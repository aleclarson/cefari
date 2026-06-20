use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LogRequest {
    pub level: LogLevel,
    pub message: String,
    pub properties_json: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Error,
    Info,
    Log,
    Warn,
}

impl LogLevel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Error => "error",
            Self::Info => "info",
            Self::Log => "log",
            Self::Warn => "warn",
        }
    }
}
