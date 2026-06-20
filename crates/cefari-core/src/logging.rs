use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::RuntimePaths;

pub const CEFARI_DAEMON_LOG_ENV: &str = "CEFARI_DAEMON_LOG";
pub const CEFARI_LOG_DATABASE_ENV: &str = "CEFARI_LOG_DATABASE";
pub const CEFARI_LOG_DATABASE_FILE_NAME: &str = "cefari.sqlite";
pub const CEFARI_LOG_SCHEMA_VERSION: u32 = 1;
pub const CEFARI_LOG_SCOPE: &str = "cefari";
pub const APP_LOG_SCOPE: &str = "app";
pub const DAEMON_LOG_SCOPE: &str = "daemon";
pub const WORKER_LOG_SCOPE_PREFIX: &str = "worker:";
pub const LOG_PROPERTY_COMPONENT: &str = "component";
pub const LOG_PROPERTY_WORKER: &str = "worker";
pub const LOG_PROPERTY_WORKER_ID: &str = "workerId";
pub const LOG_PROPERTY_CONNECTION_ID: &str = "connectionId";
pub const LOG_PROPERTY_WINDOW_ID: &str = "windowId";
const DEFAULT_RETAINED_LOG_FILES: usize = 7;
const DEFAULT_MAX_LOG_BYTES: u64 = 1_048_576;
const SECRET_KEYS: &[&str] = &["authorization", "cefari_session_token", "token"];
const ENV_SECRET_FRAGMENTS: &[&str] = &["AUTH", "KEY", "SECRET", "TOKEN"];

pub const CEFARI_LOG_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS log_entries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at TEXT NOT NULL,
  scope TEXT NOT NULL,
  level TEXT NOT NULL,
  pid INTEGER NOT NULL,
  message TEXT NOT NULL,
  properties_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS log_collapsed_values (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  kind TEXT NOT NULL,
  byte_length INTEGER NOT NULL,
  body_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS log_entries_at_idx ON log_entries(at);
CREATE INDEX IF NOT EXISTS log_entries_scope_at_idx ON log_entries(scope, at);
CREATE INDEX IF NOT EXISTS log_entries_message_at_idx ON log_entries(message, at);
"#;

/// Runtime logging inputs for the desktop shell.
///
/// `cefari-core` resolves where logs should live and what format the runtime
/// should request. Installing a global tracing subscriber remains owned by
/// `cefari-desktop`, because it must coordinate process startup and appender
/// guards with the native shell lifetime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimeLogConfig {
    pub directory: PathBuf,
    pub database: LogDatabaseConfig,
    /// Transitional file streams kept until all runtime writers move to the
    /// SQLite log database.
    pub app: LogFileConfig,
    /// Transitional file streams kept until all runtime writers move to the
    /// SQLite log database.
    pub daemon: LogFileConfig,
    /// Transitional file streams kept until all runtime writers move to the
    /// SQLite log database.
    pub rust: LogFileConfig,
}

impl RuntimeLogConfig {
    #[must_use]
    pub fn new(paths: &RuntimePaths) -> Self {
        let directory = paths.log_dir.clone();
        Self {
            app: LogFileConfig::new(
                LogStream::App,
                &directory,
                "app.log",
                LogFormat::Compact,
                LogRotation::Daily,
            ),
            daemon: LogFileConfig::new(
                LogStream::Daemon,
                &directory,
                "daemon.log",
                LogFormat::Plain,
                LogRotation::Size {
                    max_bytes: DEFAULT_MAX_LOG_BYTES,
                },
            ),
            rust: LogFileConfig::new(
                LogStream::Rust,
                &directory,
                "rust.log",
                LogFormat::Json,
                LogRotation::Daily,
            ),
            database: LogDatabaseConfig {
                directory: directory.clone(),
                file_name: CEFARI_LOG_DATABASE_FILE_NAME.to_owned(),
                schema_version: CEFARI_LOG_SCHEMA_VERSION,
            },
            directory,
        }
    }

    #[must_use]
    pub fn streams(&self) -> [&LogFileConfig; 3] {
        [&self.app, &self.daemon, &self.rust]
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogDatabaseConfig {
    pub directory: PathBuf,
    pub file_name: String,
    pub schema_version: u32,
}

impl LogDatabaseConfig {
    #[must_use]
    pub fn file_path(&self) -> PathBuf {
        self.directory.join(&self.file_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LogScope {
    Cefari,
    App,
    Daemon,
    Worker(String),
}

impl LogScope {
    #[must_use]
    pub fn as_value(&self) -> String {
        match self {
            Self::Cefari => CEFARI_LOG_SCOPE.to_owned(),
            Self::App => APP_LOG_SCOPE.to_owned(),
            Self::Daemon => DAEMON_LOG_SCOPE.to_owned(),
            Self::Worker(worker) => worker_log_scope(worker),
        }
    }
}

#[must_use]
pub fn worker_log_scope(worker: &str) -> String {
    format!("{WORKER_LOG_SCOPE_PREFIX}{worker}")
}

#[must_use]
pub fn should_redact_log_property_key(key: &str, parent_key: Option<&str>) -> bool {
    if SECRET_KEYS
        .iter()
        .any(|secret_key| key.eq_ignore_ascii_case(secret_key))
    {
        return true;
    }

    let Some(parent_key) = parent_key else {
        return false;
    };
    let parent_key = parent_key.to_ascii_uppercase();
    if parent_key != "ENV" && !parent_key.ends_with("_ENV") {
        return false;
    }

    let key = key.to_ascii_uppercase();
    ENV_SECRET_FRAGMENTS
        .iter()
        .any(|fragment| key.contains(fragment))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogFormat {
    Compact,
    Json,
    Plain,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogFileConfig {
    pub stream: LogStream,
    pub directory: PathBuf,
    pub file_name: String,
    pub format: LogFormat,
    pub rotation: LogRotation,
    pub retained_files: usize,
}

impl LogFileConfig {
    fn new(
        stream: LogStream,
        directory: &Path,
        file_name: impl Into<String>,
        format: LogFormat,
        rotation: LogRotation,
    ) -> Self {
        Self {
            stream,
            directory: directory.to_path_buf(),
            file_name: file_name.into(),
            format,
            rotation,
            retained_files: DEFAULT_RETAINED_LOG_FILES,
        }
    }

    #[must_use]
    pub fn file_path(&self) -> PathBuf {
        self.directory.join(&self.file_name)
    }

    #[must_use]
    pub fn rotated_file_prefix(&self) -> String {
        format!("{}.", self.file_name)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogRotation {
    Daily,
    Size { max_bytes: u64 },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogStream {
    App,
    Daemon,
    Rust,
}

pub fn prune_rotated_logs(config: &LogFileConfig) -> std::io::Result<()> {
    let prefix = config.rotated_file_prefix();
    let mut rotated_files = Vec::new();

    for entry in fs::read_dir(&config.directory)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if file_name.starts_with(&prefix) {
            rotated_files.push(entry.path());
        }
    }

    rotated_files.sort_by(|left, right| {
        match (
            rotated_numeric_suffix(left, config),
            rotated_numeric_suffix(right, config),
        ) {
            (Some(left), Some(right)) => right.cmp(&left),
            _ => left.cmp(right),
        }
    });
    let remove_count = rotated_files.len().saturating_sub(config.retained_files);
    for path in rotated_files.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn rotated_numeric_suffix(path: &Path, config: &LogFileConfig) -> Option<usize> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(&config.rotated_file_prefix()))
        .and_then(|suffix| suffix.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{
        APP_LOG_SCOPE, CEFARI_DAEMON_LOG_ENV, CEFARI_LOG_DATABASE_ENV,
        CEFARI_LOG_DATABASE_FILE_NAME, CEFARI_LOG_SCHEMA_SQL, CEFARI_LOG_SCHEMA_VERSION,
        DAEMON_LOG_SCOPE, DEFAULT_MAX_LOG_BYTES, LOG_PROPERTY_COMPONENT,
        LOG_PROPERTY_CONNECTION_ID, LOG_PROPERTY_WINDOW_ID, LOG_PROPERTY_WORKER,
        LOG_PROPERTY_WORKER_ID, LogFormat, LogRotation, LogScope, LogStream, RuntimeLogConfig,
        WORKER_LOG_SCOPE_PREFIX, prune_rotated_logs, should_redact_log_property_key,
        worker_log_scope,
    };
    use crate::{AppIdentity, RuntimePaths};
    use std::fs;

    #[test]
    fn derives_default_log_files_from_runtime_paths() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let config = RuntimeLogConfig::new(&paths);

        assert_eq!(config.app.stream, LogStream::App);
        assert_eq!(config.app.format, LogFormat::Compact);
        assert_eq!(config.app.rotation, LogRotation::Daily);
        assert_eq!(config.app.file_path(), paths.log_dir.join("app.log"));
        assert_eq!(
            config.daemon.rotation,
            LogRotation::Size {
                max_bytes: DEFAULT_MAX_LOG_BYTES
            }
        );
        assert_eq!(config.daemon.file_path(), paths.log_dir.join("daemon.log"));
        assert_eq!(config.rust.format, LogFormat::Json);
        assert_eq!(config.rust.file_path(), paths.log_dir.join("rust.log"));
        assert_eq!(config.streams().len(), 3);
        assert_eq!(CEFARI_DAEMON_LOG_ENV, "CEFARI_DAEMON_LOG");
    }

    #[test]
    fn derives_log_database_contract_from_runtime_paths() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let config = RuntimeLogConfig::new(&paths);

        assert_eq!(config.database.directory, paths.log_dir);
        assert_eq!(config.database.file_name, CEFARI_LOG_DATABASE_FILE_NAME);
        assert_eq!(config.database.schema_version, CEFARI_LOG_SCHEMA_VERSION);
        assert_eq!(
            config.database.file_path(),
            paths.log_dir.join("cefari.sqlite")
        );
        assert_eq!(CEFARI_LOG_DATABASE_ENV, "CEFARI_LOG_DATABASE");
    }

    #[test]
    fn defines_first_class_log_scopes_and_property_names() {
        assert_eq!(LogScope::Cefari.as_value(), "cefari");
        assert_eq!(LogScope::App.as_value(), APP_LOG_SCOPE);
        assert_eq!(LogScope::Daemon.as_value(), DAEMON_LOG_SCOPE);
        assert_eq!(
            LogScope::Worker("thumbnailer".to_owned()).as_value(),
            "worker:thumbnailer"
        );
        assert_eq!(worker_log_scope("thumbnailer"), "worker:thumbnailer");
        assert_eq!(WORKER_LOG_SCOPE_PREFIX, "worker:");

        assert_eq!(LOG_PROPERTY_COMPONENT, "component");
        assert_eq!(LOG_PROPERTY_WORKER, "worker");
        assert_eq!(LOG_PROPERTY_WORKER_ID, "workerId");
        assert_eq!(LOG_PROPERTY_CONNECTION_ID, "connectionId");
        assert_eq!(LOG_PROPERTY_WINDOW_ID, "windowId");
    }

    #[test]
    fn defines_sqlite_log_schema_contract() {
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS log_entries"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("scope TEXT NOT NULL"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("level TEXT NOT NULL"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("pid INTEGER NOT NULL"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("properties_json TEXT NOT NULL"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS log_collapsed_values"));
        assert!(CEFARI_LOG_SCHEMA_SQL.contains("log_entries_scope_at_idx"));
    }

    #[test]
    fn defines_secret_redaction_policy() {
        assert!(should_redact_log_property_key("token", None));
        assert!(should_redact_log_property_key("Authorization", None));
        assert!(!should_redact_log_property_key("path", None));
        assert!(should_redact_log_property_key(
            "OPENAI_API_KEY",
            Some("env")
        ));
        assert!(should_redact_log_property_key(
            "SERVICE_TOKEN",
            Some("worker_env")
        ));
        assert!(!should_redact_log_property_key("PATH", Some("env")));
        assert!(!should_redact_log_property_key(
            "OPENAI_API_KEY",
            Some("metadata")
        ));
    }

    #[test]
    fn prunes_old_rotated_log_files() {
        let root = std::env::temp_dir().join(format!("cefari-log-prune-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp log directory should exist");

        let mut config =
            RuntimeLogConfig::new(&RuntimePaths::resolve(&AppIdentity::cefari()).unwrap()).app;
        config.directory.clone_from(&root);
        config.retained_files = 2;

        fs::write(root.join("app.log.2026-06-01"), "").unwrap();
        fs::write(root.join("app.log.2026-06-02"), "").unwrap();
        fs::write(root.join("app.log.2026-06-03"), "").unwrap();
        fs::write(root.join("app.log"), "").unwrap();

        prune_rotated_logs(&config).expect("old rotated logs should prune");

        assert!(!root.join("app.log.2026-06-01").exists());
        assert!(root.join("app.log.2026-06-02").exists());
        assert!(root.join("app.log.2026-06-03").exists());
        assert!(root.join("app.log").exists());

        fs::remove_dir_all(root).expect("temp log directory should be removable");
    }

    #[test]
    fn prunes_oldest_numbered_rotated_log_files() {
        let root =
            std::env::temp_dir().join(format!("cefari-log-numbered-prune-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp log directory should exist");

        let mut config =
            RuntimeLogConfig::new(&RuntimePaths::resolve(&AppIdentity::cefari()).unwrap()).daemon;
        config.directory.clone_from(&root);
        config.retained_files = 2;

        fs::write(root.join("daemon.log.1"), "").unwrap();
        fs::write(root.join("daemon.log.2"), "").unwrap();
        fs::write(root.join("daemon.log.3"), "").unwrap();

        prune_rotated_logs(&config).expect("old rotated logs should prune");

        assert!(root.join("daemon.log.1").exists());
        assert!(root.join("daemon.log.2").exists());
        assert!(!root.join("daemon.log.3").exists());

        fs::remove_dir_all(root).expect("temp log directory should be removable");
    }
}
