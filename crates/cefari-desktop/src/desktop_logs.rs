use anyhow::{Context, Result};
use cefari_core::{
    APP_LOG_SCOPE, LOG_PROPERTY_WINDOW_ID, LogRequest, RuntimeLogConfig, RuntimePaths,
};
use serde_json::{Map, Value};

use crate::logging;

pub(crate) fn append_app_log(
    paths: &RuntimePaths,
    source_window_id: Option<&str>,
    request: &LogRequest,
) -> Result<()> {
    let mut properties = parse_properties(&request.properties_json)?;
    if let Some(window_id) = source_window_id {
        properties.insert(
            LOG_PROPERTY_WINDOW_ID.to_owned(),
            Value::String(window_id.to_owned()),
        );
    }

    logging::append_log_entry(
        &RuntimeLogConfig::new(paths).database.file_path(),
        APP_LOG_SCOPE,
        request.level.as_str(),
        &request.message,
        Value::Object(properties),
    )
}

fn parse_properties(source: &str) -> Result<Map<String, Value>> {
    let value: Value = serde_json::from_str(source).context("log propertiesJson must be JSON")?;
    match value {
        Value::Object(properties) => Ok(properties),
        _ => anyhow::bail!("log propertiesJson must be a JSON object"),
    }
}
