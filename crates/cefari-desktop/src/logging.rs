use std::{
    collections::VecDeque,
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::{
    CEFARI_LOG_SCHEMA_SQL, CEFARI_LOG_SCOPE, RuntimeLogConfig, RuntimePaths,
    should_redact_log_property_key,
};
use rusqlite::{Connection, params};
use serde_json::{Map, Number, Value};
use tracing::{Event, Level, Subscriber, field::Field, field::Visit};
use tracing_subscriber::{
    Layer, layer::Context as LayerContext, layer::SubscriberExt, util::SubscriberInitExt,
};

pub(crate) struct LogGuards {
    router: Arc<LogRouter>,
}

type SharedLogDatabase = Arc<Mutex<Connection>>;
const SENTRY_LOG_BATCH_SIZE: usize = 10;
const SENTRY_LOG_QUEUE_LIMIT: usize = 512;
const SENTRY_RETRY_BACKOFF: Duration = Duration::from_secs(1);
static NEXT_LOG_ID: AtomicI64 = AtomicI64::new(1);

#[derive(Debug, Clone)]
pub(crate) struct SentryLogSinkConfig {
    pub(crate) dsn: String,
    pub(crate) environment: Option<String>,
    pub(crate) release: Option<String>,
    pub(crate) level: String,
    pub(crate) sample_rate: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct LogEvent {
    pub(crate) id: i64,
    pub(crate) scope: String,
    pub(crate) level: String,
    pub(crate) message: String,
    pub(crate) properties: Value,
    pub(crate) pid: i64,
    pub(crate) at: String,
}

impl LogEvent {
    pub(crate) fn new(
        scope: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
        properties: Value,
    ) -> Self {
        Self {
            id: next_log_id(),
            scope: scope.into(),
            level: level.into(),
            message: message.into(),
            properties,
            pid: i64::from(std::process::id()),
            at: iso_timestamp(),
        }
    }
}

fn next_log_id() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    millis.saturating_mul(10_000) + NEXT_LOG_ID.fetch_add(1, Ordering::Relaxed) % 10_000
}

pub(crate) trait LogSink: Send + Sync {
    fn append(&self, event: &LogEvent) -> Result<()>;

    fn flush(&self) -> Result<()> {
        Ok(())
    }
}

pub(crate) struct LogRouter {
    sinks: Vec<Arc<dyn LogSink>>,
}

impl LogRouter {
    pub(crate) fn new(sinks: Vec<Arc<dyn LogSink>>) -> Self {
        Self { sinks }
    }

    pub(crate) fn with_local_database(database_path: &Path) -> Result<Self> {
        Ok(Self::new(vec![
            Arc::new(SqliteLogSink::open(database_path)?) as Arc<dyn LogSink>,
        ]))
    }

    pub(crate) fn with_optional_local_database(
        database_path: &Path,
        enabled: bool,
    ) -> Result<Self> {
        if enabled {
            Self::with_local_database(database_path)
        } else {
            Ok(Self::new(Vec::new()))
        }
    }

    pub(crate) fn route(&self, event: &LogEvent) -> Result<()> {
        for sink in &self.sinks {
            sink.append(event)?;
        }
        Ok(())
    }

    pub(crate) fn flush(&self) -> Result<()> {
        for sink in &self.sinks {
            sink.flush()?;
        }
        Ok(())
    }
}

pub(crate) struct SqliteLogSink {
    database: SharedLogDatabase,
}

impl SqliteLogSink {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            database: open_log_database(path)?,
        })
    }

    fn from_database(database: SharedLogDatabase) -> Self {
        Self { database }
    }
}

impl LogSink for SqliteLogSink {
    fn append(&self, event: &LogEvent) -> Result<()> {
        let connection = self
            .database
            .lock()
            .map_err(|error| anyhow::anyhow!("failed to lock log database: {error}"))?;
        insert_log_entry(&connection, event).context("failed to insert log entry")
    }
}

pub(crate) trait SentryTransport: Send + Sync {
    fn send_envelope(&self, url: &str, body: &str) -> Result<()>;
}

struct UreqSentryTransport;

impl SentryTransport for UreqSentryTransport {
    fn send_envelope(&self, url: &str, body: &str) -> Result<()> {
        ureq::post(url)
            .header("content-type", "application/x-sentry-envelope")
            .send(body)
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("failed to send Sentry log envelope: {error}"))
    }
}

pub(crate) struct SentryLogSink {
    config: SentryLogSinkConfig,
    endpoint: String,
    transport: Arc<dyn SentryTransport>,
    queue: Mutex<SentryQueue>,
}

struct SentryQueue {
    events: VecDeque<LogEvent>,
    retry_after: Option<Instant>,
}

impl SentryLogSink {
    pub(crate) fn new(config: SentryLogSinkConfig) -> Result<Self> {
        Self::with_transport(config, Arc::new(UreqSentryTransport))
    }

    pub(crate) fn with_transport(
        config: SentryLogSinkConfig,
        transport: Arc<dyn SentryTransport>,
    ) -> Result<Self> {
        let endpoint = sentry_envelope_endpoint(&config.dsn)?;
        Ok(Self {
            config,
            endpoint,
            transport,
            queue: Mutex::new(SentryQueue {
                events: VecDeque::new(),
                retry_after: None,
            }),
        })
    }

    fn flush_locked(&self, queue: &mut SentryQueue, force: bool) -> Result<()> {
        if queue.events.is_empty() {
            return Ok(());
        }
        if !force && queue.events.len() < SENTRY_LOG_BATCH_SIZE {
            return Ok(());
        }
        if !force
            && queue
                .retry_after
                .is_some_and(|retry_after| retry_after > Instant::now())
        {
            return Ok(());
        }

        let batch_size = queue.events.len().min(SENTRY_LOG_BATCH_SIZE);
        let batch: Vec<LogEvent> = queue.events.drain(..batch_size).collect();
        let body = sentry_log_envelope(&self.config, &batch)?;
        match self.transport.send_envelope(&self.endpoint, &body) {
            Ok(()) => {
                queue.retry_after = None;
                Ok(())
            }
            Err(error) => {
                for event in batch.into_iter().rev() {
                    queue.events.push_front(event);
                }
                queue.retry_after = Some(Instant::now() + SENTRY_RETRY_BACKOFF);
                Err(error)
            }
        }
    }
}

impl LogSink for SentryLogSink {
    fn append(&self, event: &LogEvent) -> Result<()> {
        if !sentry_level_enabled(&event.level, &self.config.level)
            || !sample_event(event, self.config.sample_rate)
        {
            return Ok(());
        }

        let mut queue = self
            .queue
            .lock()
            .map_err(|error| anyhow::anyhow!("failed to lock Sentry log queue: {error}"))?;
        if queue.events.len() >= SENTRY_LOG_QUEUE_LIMIT {
            queue.events.pop_front();
        }
        queue.events.push_back(event.clone());
        if let Err(error) = self.flush_locked(&mut queue, false) {
            eprintln!("{error}");
        }
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|error| anyhow::anyhow!("failed to lock Sentry log queue: {error}"))?;
        while !queue.events.is_empty() {
            self.flush_locked(&mut queue, true)?;
        }
        Ok(())
    }
}

impl LogGuards {
    pub(crate) fn router(&self) -> Arc<LogRouter> {
        self.router.clone()
    }
}

pub(crate) fn init_logging(
    paths: &RuntimePaths,
    local_storage_enabled: bool,
    sentry: Option<SentryLogSinkConfig>,
) -> Result<LogGuards> {
    let log_config = RuntimeLogConfig::new(paths);
    fs::create_dir_all(&log_config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            log_config.directory.display()
        )
    })?;

    let mut sinks: Vec<Arc<dyn LogSink>> = Vec::new();
    if local_storage_enabled {
        sinks.push(Arc::new(SqliteLogSink::open(
            &log_config.database.file_path(),
        )?));
    }
    if let Some(sentry) = sentry {
        sinks.push(Arc::new(SentryLogSink::new(sentry)?));
    }
    let router = Arc::new(LogRouter::new(sinks));
    let layer = RoutedLogLayer::new(router.clone());

    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(LogGuards { router })
}

impl Drop for LogGuards {
    fn drop(&mut self) {
        if let Err(error) = self.router.flush() {
            eprintln!("failed to flush Cefari log router: {error}");
        }
    }
}

fn open_log_database(path: &Path) -> Result<SharedLogDatabase> {
    let connection = Connection::open(path)
        .with_context(|| format!("failed to open log database at {}", path.display()))?;
    initialize_log_database(&connection)
        .with_context(|| format!("failed to initialize log database at {}", path.display()))?;
    Ok(Arc::new(Mutex::new(connection)))
}

fn initialize_log_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch(CEFARI_LOG_SCHEMA_SQL)
}

#[derive(Clone)]
struct RoutedLogLayer {
    router: Arc<LogRouter>,
}

impl RoutedLogLayer {
    fn new(router: Arc<LogRouter>) -> Self {
        Self { router }
    }
}

impl<S> Layer<S> for RoutedLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: LayerContext<'_, S>) {
        let metadata = event.metadata();
        let mut fields = LogEventFields::default();
        event.record(&mut fields);

        let message = fields.message.unwrap_or_else(|| metadata.name().to_owned());
        let mut properties = fields.properties;
        properties.insert(
            "target".to_owned(),
            Value::String(metadata.target().to_owned()),
        );
        if let Some(module_path) = metadata.module_path() {
            properties.insert(
                "modulePath".to_owned(),
                Value::String(module_path.to_owned()),
            );
        }
        if let Some(file) = metadata.file() {
            properties.insert("file".to_owned(), Value::String(file.to_owned()));
        }
        if let Some(line) = metadata.line() {
            properties.insert("line".to_owned(), Value::Number(Number::from(line)));
        }

        let event = LogEvent::new(
            CEFARI_LOG_SCOPE,
            level_name(metadata.level()),
            message,
            Value::Object(properties),
        );

        if let Err(error) = self.router.route(&event) {
            eprintln!("failed to route Cefari runtime log: {error}");
        }
    }
}

pub(crate) fn append_log_entry(
    database_path: &Path,
    scope: &str,
    level: &str,
    message: &str,
    properties: Value,
) -> Result<()> {
    let router = LogRouter::with_local_database(database_path)?;
    router.route(&LogEvent::new(scope, level, message, properties))
}

fn insert_log_entry(connection: &Connection, event: &LogEvent) -> rusqlite::Result<()> {
    let properties_json = redact_log_value(&event.properties, None).to_string();
    connection.execute(
        "INSERT INTO log_entries (id, at, scope, level, pid, message, properties_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event.id,
            event.at,
            event.scope,
            event.level,
            event.pid,
            event.message,
            properties_json
        ],
    )?;
    Ok(())
}

fn sentry_envelope_endpoint(dsn: &str) -> Result<String> {
    let (scheme, rest) = dsn
        .split_once("://")
        .ok_or_else(|| anyhow::anyhow!("Sentry DSN must include a URL scheme"))?;
    let public_key_host_path = rest
        .split_once('@')
        .map_or(rest, |(_, host_path)| host_path);
    let (host, path) = public_key_host_path
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Sentry DSN must include a project id"))?;
    let mut segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let project_id = segments
        .pop()
        .ok_or_else(|| anyhow::anyhow!("Sentry DSN must include a project id"))?;
    let prefix = if segments.is_empty() {
        String::new()
    } else {
        format!("/{}", segments.join("/"))
    };
    Ok(format!(
        "{scheme}://{host}{prefix}/api/{project_id}/envelope/"
    ))
}

fn sentry_log_envelope(config: &SentryLogSinkConfig, events: &[LogEvent]) -> Result<String> {
    let header = serde_json::json!({
        "dsn": config.dsn,
        "sent_at": iso_timestamp(),
    });
    let item_header = serde_json::json!({
        "type": "log",
        "item_count": events.len(),
        "content_type": "application/vnd.sentry.items.log+json",
    });
    let items: Vec<Value> = events
        .iter()
        .map(|event| sentry_log_payload(config, event))
        .collect::<Result<_>>()?;
    let payload = serde_json::json!({ "items": items });
    Ok(format!("{header}\n{item_header}\n{payload}\n"))
}

fn sentry_log_payload(config: &SentryLogSinkConfig, event: &LogEvent) -> Result<Value> {
    let mut attributes = Map::new();
    attributes.insert(
        "cefari.scope".to_owned(),
        sentry_attribute(Value::String(event.scope.clone())),
    );
    attributes.insert(
        "cefari.pid".to_owned(),
        sentry_attribute(Value::Number(Number::from(event.pid))),
    );
    attributes.insert(
        "cefari.log_id".to_owned(),
        sentry_attribute(Value::Number(Number::from(event.id))),
    );
    if let Some(environment) = &config.environment {
        attributes.insert(
            "environment".to_owned(),
            sentry_attribute(Value::String(environment.clone())),
        );
    }
    if let Some(release) = &config.release {
        attributes.insert(
            "release".to_owned(),
            sentry_attribute(Value::String(release.clone())),
        );
    }
    if let Value::Object(properties) = redact_log_value(&event.properties, None) {
        for (key, value) in properties {
            attributes.insert(key, sentry_attribute(value));
        }
    }

    let level = sentry_level(&event.level);
    Ok(serde_json::json!({
        "timestamp": iso_timestamp_seconds(&event.at)?,
        "trace_id": "00000000000000000000000000000000",
        "level": level,
        "body": event.message,
        "severity_number": sentry_severity_number(level),
        "attributes": attributes,
    }))
}

fn sentry_attribute(value: Value) -> Value {
    let type_name = match value {
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "string",
        Value::Null => "string",
    };
    let value = match value {
        Value::Array(_) | Value::Object(_) | Value::Null => Value::String(value.to_string()),
        value => value,
    };
    serde_json::json!({ "value": value, "type": type_name })
}

fn sentry_level(level: &str) -> &'static str {
    match level {
        "debug" => "debug",
        "warn" => "warn",
        "error" => "error",
        "log" | "info" => "info",
        _ => "info",
    }
}

fn sentry_level_enabled(level: &str, minimum: &str) -> bool {
    level_rank(sentry_level(level)) >= level_rank(sentry_level(minimum))
}

fn level_rank(level: &str) -> u8 {
    match level {
        "debug" => 1,
        "info" => 2,
        "warn" => 3,
        "error" => 4,
        "fatal" => 5,
        _ => 2,
    }
}

fn sentry_severity_number(level: &str) -> u8 {
    match level {
        "debug" => 5,
        "info" => 9,
        "warn" => 13,
        "error" => 17,
        "fatal" => 21,
        _ => 9,
    }
}

fn sample_event(event: &LogEvent, sample_rate: f64) -> bool {
    if sample_rate >= 1.0 {
        return true;
    }
    if sample_rate <= 0.0 {
        return false;
    }
    let mut hash = 0_u64;
    for byte in event.at.bytes().chain(event.message.bytes()) {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    (hash % 10_000) as f64 / 10_000.0 < sample_rate
}

fn redact_log_value(value: &Value, parent_key: Option<&str>) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_log_value(item, parent_key))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    let value = if should_redact_log_property_key(key, parent_key) {
                        Value::String("[redacted]".to_owned())
                    } else {
                        redact_log_value(value, Some(key))
                    };
                    (key.clone(), value)
                })
                .collect(),
        ),
        value => value.clone(),
    }
}

#[derive(Default)]
struct LogEventFields {
    message: Option<String>,
    properties: Map<String, Value>,
}

impl LogEventFields {
    fn record_value(&mut self, field: &Field, value: Value) {
        if field.name() == "message" {
            self.message = Some(match value {
                Value::String(value) => value,
                value => value.to_string(),
            });
            return;
        }

        self.properties.insert(field.name().to_owned(), value);
    }
}

impl Visit for LogEventFields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record_value(field, Value::String(format!("{value:?}")));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, Value::Number(Number::from(value)));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, Value::Number(Number::from(value)));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, Value::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, Value::String(value.to_owned()));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record_value(field, Value::String(value.to_string()));
    }
}

fn level_name(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "error",
        Level::WARN => "warn",
        Level::INFO => "info",
        Level::DEBUG => "debug",
        Level::TRACE => "debug",
    }
}

fn iso_timestamp() -> String {
    let now = std::time::SystemTime::now();
    match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!(
            "{}.{:03}Z",
            chrono_like_seconds(duration.as_secs()),
            duration.subsec_millis()
        ),
        Err(_) => "1970-01-01T00:00:00.000Z".to_owned(),
    }
}

fn iso_timestamp_seconds(value: &str) -> Result<f64> {
    if value.len() < "2026-06-20T13:00:00Z".len() {
        anyhow::bail!("invalid ISO timestamp");
    }
    let year: i32 = value[0..4].parse()?;
    let month: u32 = value[5..7].parse()?;
    let day: u32 = value[8..10].parse()?;
    let hour: u64 = value[11..13].parse()?;
    let minute: u64 = value[14..16].parse()?;
    let second: u64 = value[17..19].parse()?;
    let millis = value
        .split_once('.')
        .and_then(|(_, fraction)| fraction.get(0..3))
        .and_then(|fraction| fraction.parse::<u64>().ok())
        .unwrap_or(0);
    let days = days_from_civil(year, month, day);
    Ok(
        (days as u64 * 86_400 + hour * 3_600 + minute * 60 + second) as f64
            + millis as f64 / 1000.0,
    )
}

fn chrono_like_seconds(seconds: u64) -> String {
    let days = seconds / 86_400;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::{
        LogEvent, LogRouter, LogSink, RoutedLogLayer, SENTRY_LOG_BATCH_SIZE, SentryLogSink,
        SentryLogSinkConfig, SentryTransport, SqliteLogSink, append_log_entry,
        initialize_log_database, iso_timestamp, sentry_envelope_endpoint, sentry_log_envelope,
    };
    use anyhow::Result;
    use cefari_core::CEFARI_LOG_SCOPE;
    use rusqlite::Connection;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use tracing::info;
    use tracing_subscriber::{Layer, layer::SubscriberExt};

    #[test]
    fn initializes_log_database_schema() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        initialize_log_database(&connection).expect("schema should initialize");

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('log_entries', 'log_collapsed_values')",
                [],
                |row| row.get(0),
            )
            .expect("schema table count");

        assert_eq!(count, 2);
    }

    #[test]
    fn writes_runtime_tracing_events_to_sqlite() {
        let connection = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory database"),
        ));
        initialize_log_database(&connection.lock().unwrap()).expect("schema should initialize");
        let router = Arc::new(LogRouter::new(vec![Arc::new(
            SqliteLogSink::from_database(connection.clone()),
        )]));
        let layer = RoutedLogLayer::new(router).boxed();
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            info!(
                component = "ipc",
                window_id = "main",
                duration_ms = 38_u64,
                "ipc.response_sent"
            );
        });

        let connection = connection.lock().unwrap();
        let row = connection
            .query_row(
                "SELECT scope, level, pid, message, properties_json FROM log_entries",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .expect("log row should exist");

        assert_eq!(row.0, CEFARI_LOG_SCOPE);
        assert_eq!(row.1, "info");
        assert!(row.2 > 0);
        assert_eq!(row.3, "ipc.response_sent");

        let properties: Value = serde_json::from_str(&row.4).expect("properties should be json");
        assert_eq!(properties["component"], "ipc");
        assert_eq!(properties["window_id"], "main");
        assert_eq!(properties["duration_ms"], 38);
        assert!(properties["target"].is_string());
    }

    #[test]
    fn appends_explicit_log_entries_to_sqlite() {
        let root = std::env::temp_dir().join(format!("cefari-app-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp dir should exist");
        let database_path = root.join("cefari.sqlite");

        append_log_entry(
            &database_path,
            "app",
            "warn",
            "app.warning",
            serde_json::json!({
                "windowId": "main",
                "token": "secret",
                "env": {
                    "OPENAI_API_KEY": "secret",
                    "PATH": "/bin"
                }
            }),
        )
        .expect("app log should append");

        let connection = Connection::open(&database_path).expect("database should open");
        let row = connection
            .query_row(
                "SELECT scope, level, message, properties_json FROM log_entries",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("log row should exist");

        assert_eq!(row.0, "app");
        assert_eq!(row.1, "warn");
        assert_eq!(row.2, "app.warning");
        let properties: Value = serde_json::from_str(&row.3).expect("properties should be json");
        assert_eq!(properties["windowId"], "main");
        assert_eq!(properties["token"], "[redacted]");
        assert_eq!(properties["env"]["OPENAI_API_KEY"], "[redacted]");
        assert_eq!(properties["env"]["PATH"], "/bin");

        std::fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn routes_one_event_to_local_sqlite() {
        let connection = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory database"),
        ));
        initialize_log_database(&connection.lock().unwrap()).expect("schema should initialize");
        let router = LogRouter::new(vec![Arc::new(SqliteLogSink::from_database(
            connection.clone(),
        ))]);

        router
            .route(&LogEvent::new(
                "app",
                "info",
                "app.ready",
                serde_json::json!({ "windowId": "main" }),
            ))
            .expect("event should route");

        let connection = connection.lock().unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM log_entries", [], |row| row.get(0))
            .expect("row count should load");
        assert_eq!(count, 1);
    }

    #[test]
    fn local_disabled_does_not_create_sqlite_database() {
        let root =
            std::env::temp_dir().join(format!("cefari-disabled-local-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp dir should exist");
        let database_path = root.join("cefari.sqlite");
        let router =
            LogRouter::with_optional_local_database(&database_path, false).expect("router");

        router
            .route(&LogEvent::new(
                "app",
                "info",
                "app.ready",
                Value::Object(Default::default()),
            ))
            .expect("event should route with no local sink");

        assert!(!database_path.exists());
        std::fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn local_sink_redacts_secret_properties() {
        let connection = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory database"),
        ));
        initialize_log_database(&connection.lock().unwrap()).expect("schema should initialize");
        let router = LogRouter::new(vec![Arc::new(SqliteLogSink::from_database(
            connection.clone(),
        ))]);

        router
            .route(&LogEvent::new(
                "app",
                "warn",
                "app.warning",
                serde_json::json!({
                    "token": "secret",
                    "env": {
                        "OPENAI_API_KEY": "secret",
                        "PATH": "/bin"
                    }
                }),
            ))
            .expect("event should route");

        let connection = connection.lock().unwrap();
        let properties_json: String = connection
            .query_row("SELECT properties_json FROM log_entries", [], |row| {
                row.get(0)
            })
            .expect("properties should load");
        let properties: Value =
            serde_json::from_str(&properties_json).expect("properties should parse");
        assert_eq!(properties["token"], "[redacted]");
        assert_eq!(properties["env"]["OPENAI_API_KEY"], "[redacted]");
        assert_eq!(properties["env"]["PATH"], "/bin");
    }

    #[derive(Default)]
    struct CapturingSink {
        events: Mutex<Vec<LogEvent>>,
    }

    impl LogSink for CapturingSink {
        fn append(&self, event: &LogEvent) -> Result<()> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn routes_events_to_multiple_sinks() {
        let first = Arc::new(CapturingSink::default());
        let second = Arc::new(CapturingSink::default());
        let router = LogRouter::new(vec![first.clone(), second.clone()]);

        router
            .route(&LogEvent::new(
                "daemon",
                "log",
                "daemon ready",
                Value::Object(Default::default()),
            ))
            .expect("event should route");

        assert_eq!(first.events.lock().unwrap().len(), 1);
        assert_eq!(second.events.lock().unwrap().len(), 1);
    }

    #[derive(Default)]
    struct CapturingTransport {
        envelopes: Mutex<Vec<(String, String)>>,
        fail_next: Mutex<bool>,
    }

    impl SentryTransport for CapturingTransport {
        fn send_envelope(&self, url: &str, body: &str) -> Result<()> {
            let mut fail_next = self.fail_next.lock().unwrap();
            if *fail_next {
                *fail_next = false;
                anyhow::bail!("transport down");
            }
            self.envelopes
                .lock()
                .unwrap()
                .push((url.to_owned(), body.to_owned()));
            Ok(())
        }
    }

    fn sentry_config() -> SentryLogSinkConfig {
        SentryLogSinkConfig {
            dsn: "https://public@sentry.invalid/42".to_owned(),
            environment: Some("test".to_owned()),
            release: Some("cefari@0.1.0".to_owned()),
            level: "info".to_owned(),
            sample_rate: 1.0,
        }
    }

    #[test]
    fn builds_sentry_envelope_endpoint_from_dsn() {
        assert_eq!(
            sentry_envelope_endpoint("https://public@sentry.invalid/42").unwrap(),
            "https://sentry.invalid/api/42/envelope/"
        );
        assert_eq!(
            sentry_envelope_endpoint("https://public@sentry.invalid/prefix/42").unwrap(),
            "https://sentry.invalid/prefix/api/42/envelope/"
        );
    }

    #[test]
    fn maps_log_events_to_sentry_log_envelopes() {
        let event = LogEvent {
            id: 42,
            scope: "worker:thumbnailer".to_owned(),
            level: "log".to_owned(),
            message: "thumbnail.ready".to_owned(),
            properties: serde_json::json!({ "durationMs": 17, "token": "secret" }),
            pid: 1234,
            at: "2026-01-02T03:04:05.250Z".to_owned(),
        };

        let envelope = sentry_log_envelope(&sentry_config(), &[event]).expect("envelope");
        let lines: Vec<&str> = envelope.lines().collect();
        assert_eq!(lines.len(), 3);
        let item_header: Value = serde_json::from_str(lines[1]).expect("item header");
        let payload: Value = serde_json::from_str(lines[2]).expect("payload");

        assert_eq!(item_header["type"], "log");
        assert_eq!(
            item_header["content_type"],
            "application/vnd.sentry.items.log+json"
        );
        let log = &payload["items"][0];
        assert_eq!(log["level"], "info");
        assert_eq!(log["body"], "thumbnail.ready");
        assert_eq!(log["timestamp"], 1767323045.25);
        assert_eq!(
            log["attributes"]["cefari.scope"],
            serde_json::json!({ "value": "worker:thumbnailer", "type": "string" })
        );
        assert_eq!(
            log["attributes"]["cefari.log_id"],
            serde_json::json!({ "value": 42, "type": "number" })
        );
        assert_eq!(
            log["attributes"]["token"],
            serde_json::json!({ "value": "[redacted]", "type": "string" })
        );
    }

    #[test]
    fn sentry_sink_batches_and_flushes_events() {
        let transport = Arc::new(CapturingTransport::default());
        let sink = SentryLogSink::with_transport(sentry_config(), transport.clone()).expect("sink");

        for index in 0..SENTRY_LOG_BATCH_SIZE {
            sink.append(&LogEvent::new(
                "app",
                "info",
                format!("event.{index}"),
                Value::Object(Default::default()),
            ))
            .expect("append should not fail");
        }

        let envelopes = transport.envelopes.lock().unwrap();
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].0, "https://sentry.invalid/api/42/envelope/");
    }

    #[test]
    fn sentry_sink_retries_failed_batches_on_flush() {
        let transport = Arc::new(CapturingTransport::default());
        *transport.fail_next.lock().unwrap() = true;
        let sink = SentryLogSink::with_transport(sentry_config(), transport.clone()).expect("sink");

        for index in 0..SENTRY_LOG_BATCH_SIZE {
            sink.append(&LogEvent::new(
                "app",
                "info",
                format!("event.{index}"),
                Value::Object(Default::default()),
            ))
            .expect("append should not fail");
        }
        assert_eq!(transport.envelopes.lock().unwrap().len(), 0);

        sink.flush().expect("flush retries queued batch");
        assert_eq!(transport.envelopes.lock().unwrap().len(), 1);
    }

    #[test]
    fn router_flush_flushes_sentry_sink() {
        let transport = Arc::new(CapturingTransport::default());
        let sink = Arc::new(
            SentryLogSink::with_transport(sentry_config(), transport.clone()).expect("sink"),
        );
        let router = LogRouter::new(vec![sink]);

        router
            .route(&LogEvent::new(
                "app",
                "warn",
                "flush.me",
                Value::Object(Default::default()),
            ))
            .expect("route should not fail");
        assert_eq!(transport.envelopes.lock().unwrap().len(), 0);

        router.flush().expect("flush should send queued event");
        assert_eq!(transport.envelopes.lock().unwrap().len(), 1);
    }

    #[test]
    fn formats_current_time_as_iso_utc() {
        let timestamp = iso_timestamp();

        assert_eq!(timestamp.len(), "2026-06-20T13:00:00.000Z".len());
        assert!(timestamp.ends_with('Z'));
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[10..11], "T");
    }
}
