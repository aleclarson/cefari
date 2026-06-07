use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::RuntimePaths;

pub const CEFARI_DAEMON_LOG_ENV: &str = "CEFARI_DAEMON_LOG";
const DEFAULT_RETAINED_LOG_FILES: usize = 7;
const DEFAULT_MAX_LOG_BYTES: u64 = 1_048_576;

/// Runtime logging inputs for the desktop shell.
///
/// `cefari-core` resolves where logs should live and what format the runtime
/// should request. Installing a global tracing subscriber remains owned by
/// `cefari-desktop`, because it must coordinate process startup and appender
/// guards with the native shell lifetime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimeLogConfig {
    pub directory: PathBuf,
    pub app: LogFileConfig,
    pub daemon: LogFileConfig,
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
            directory,
        }
    }

    #[must_use]
    pub fn streams(&self) -> [&LogFileConfig; 3] {
        [&self.app, &self.daemon, &self.rust]
    }
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
        CEFARI_DAEMON_LOG_ENV, DEFAULT_MAX_LOG_BYTES, LogFormat, LogRotation, LogStream,
        RuntimeLogConfig, prune_rotated_logs,
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
