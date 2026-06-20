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
    _database: SharedLogDatabase,
}

type SharedLogDatabase = Arc<Mutex<Connection>>;

pub(crate) fn init_logging(paths: &RuntimePaths) -> Result<LogGuards> {
    let log_config = RuntimeLogConfig::new(paths);
    fs::create_dir_all(&log_config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            log_config.directory.display()
        )
    })?;

    let database = open_log_database(&log_config.database.file_path())?;
    let layer = SqliteLogLayer::new(database.clone());

    tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(LogGuards {
        _database: database,
    })
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
struct SqliteLogLayer {
    database: SharedLogDatabase,
}

impl SqliteLogLayer {
    fn new(database: SharedLogDatabase) -> Self {
        Self { database }
    }
}

impl<S> Layer<S> for SqliteLogLayer
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

        let input = LogEntryInput {
            scope: CEFARI_LOG_SCOPE,
            level: level_name(metadata.level()),
            message: &message,
            properties: Value::Object(properties),
            pid: i64::from(std::process::id()),
            at: iso_timestamp(),
        };

        match self.database.lock() {
            Ok(connection) => {
                if let Err(error) = insert_log_entry(&connection, &input) {
                    eprintln!("failed to write Cefari runtime log: {error}");
                }
            }
            Err(error) => {
                eprintln!("failed to lock Cefari runtime log database: {error}");
            }
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
    let database = open_log_database(database_path)?;
    let connection = database
        .lock()
        .map_err(|error| anyhow::anyhow!("failed to lock log database: {error}"))?;
    let input = LogEntryInput {
        scope,
        level,
        message,
        properties,
        pid: i64::from(std::process::id()),
        at: iso_timestamp(),
    };
    insert_log_entry(&connection, &input).context("failed to insert log entry")
}

struct LogEntryInput<'a> {
    scope: &'a str,
    level: &'a str,
    message: &'a str,
    properties: Value,
    pid: i64,
    at: String,
}

fn insert_log_entry(connection: &Connection, input: &LogEntryInput<'_>) -> rusqlite::Result<()> {
    let properties_json = redact_log_value(&input.properties, None).to_string();
    connection.execute(
        "INSERT INTO log_entries (at, scope, level, pid, message, properties_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.at,
            input.scope,
            input.level,
            input.pid,
            input.message,
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
    use super::{SqliteLogLayer, append_log_entry, initialize_log_database, iso_timestamp};
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
        let layer = SqliteLogLayer::new(connection.clone()).boxed();
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
    fn formats_current_time_as_iso_utc() {
        let timestamp = iso_timestamp();

        assert_eq!(timestamp.len(), "2026-06-20T13:00:00.000Z".len());
        assert!(timestamp.ends_with('Z'));
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[10..11], "T");
    }
}
