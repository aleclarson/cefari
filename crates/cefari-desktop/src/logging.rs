use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
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

#[derive(Debug, Clone)]
pub(crate) struct LogEvent {
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
            scope: scope.into(),
            level: level.into(),
            message: message.into(),
            properties,
            pid: i64::from(std::process::id()),
            at: iso_timestamp(),
        }
    }
}

pub(crate) trait LogSink: Send + Sync {
    fn append(&self, event: &LogEvent) -> Result<()>;
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

impl LogGuards {
    pub(crate) fn router(&self) -> Arc<LogRouter> {
        self.router.clone()
    }
}

pub(crate) fn init_logging(paths: &RuntimePaths, local_storage_enabled: bool) -> Result<LogGuards> {
    let log_config = RuntimeLogConfig::new(paths);
    fs::create_dir_all(&log_config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            log_config.directory.display()
        )
    })?;

    let router = Arc::new(LogRouter::with_optional_local_database(
        &log_config.database.file_path(),
        local_storage_enabled,
    )?);
    let layer = RoutedLogLayer::new(router.clone());

    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(LogGuards { router })
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
        "INSERT INTO log_entries (at, scope, level, pid, message, properties_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
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

fn chrono_like_seconds(seconds: u64) -> String {
    let days = seconds / 86_400;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
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
        LogEvent, LogRouter, LogSink, RoutedLogLayer, SqliteLogSink, append_log_entry,
        initialize_log_database, iso_timestamp,
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

    #[test]
    fn formats_current_time_as_iso_utc() {
        let timestamp = iso_timestamp();

        assert_eq!(timestamp.len(), "2026-06-20T13:00:00.000Z".len());
        assert!(timestamp.ends_with('Z'));
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[10..11], "T");
    }
}
