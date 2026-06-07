use std::path::PathBuf;

use crate::RuntimePaths;

/// Runtime logging inputs for the desktop shell.
///
/// `cefari-core` resolves where logs should live and what format the runtime
/// should request. Installing a global tracing subscriber remains owned by
/// `cefari-desktop`, because it must coordinate process startup and appender
/// guards with the native shell lifetime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimeLogConfig {
    pub directory: PathBuf,
    pub file_name: String,
    pub format: LogFormat,
}

impl RuntimeLogConfig {
    #[must_use]
    pub fn new(paths: &RuntimePaths) -> Self {
        Self {
            directory: paths.log_dir.clone(),
            file_name: "cefari.log".to_owned(),
            format: LogFormat::Pretty,
        }
    }

    #[must_use]
    pub fn file_path(&self) -> PathBuf {
        self.directory.join(&self.file_name)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[cfg(test)]
mod tests {
    use super::{LogFormat, RuntimeLogConfig};
    use crate::{AppIdentity, RuntimePaths};

    #[test]
    fn derives_default_log_file_from_runtime_paths() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let config = RuntimeLogConfig::new(&paths);

        assert_eq!(config.format, LogFormat::Pretty);
        assert_eq!(config.file_name, "cefari.log");
        assert_eq!(config.file_path(), paths.log_dir.join("cefari.log"));
    }
}
