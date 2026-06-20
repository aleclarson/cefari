use anyhow::{Context, Result};
use cefari_core::{APP_LOG_SCOPE, LOG_PROPERTY_WINDOW_ID, LogRequest};
use serde_json::{Map, Value};

use crate::logging;

pub(crate) fn append_app_log(
    router: &logging::LogRouter,
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

    router.route(&logging::LogEvent::new(
        APP_LOG_SCOPE,
        request.level.as_str(),
        request.message.clone(),
        Value::Object(properties),
    ))
}

fn parse_properties(source: &str) -> Result<Map<String, Value>> {
    let value: Value = serde_json::from_str(source).context("log propertiesJson must be JSON")?;
    match value {
        Value::Object(properties) => Ok(properties),
        _ => anyhow::bail!("log propertiesJson must be a JSON object"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use anyhow::Result;
    use cefari_core::{LogLevel, LogRequest};
    use serde_json::Value;

    use super::append_app_log;
    use crate::logging::{LogEvent, LogRouter, LogSink};

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
    fn routes_app_log_requests_through_router() {
        let sink = Arc::new(CapturingSink::default());
        let router = LogRouter::new(vec![sink.clone()]);

        append_app_log(
            &router,
            Some("main"),
            &LogRequest {
                level: LogLevel::Warn,
                message: "settings.saved".to_owned(),
                properties_json: r#"{"section":"profile"}"#.to_owned(),
            },
        )
        .expect("app log should route");

        let events = sink.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].scope, "app");
        assert_eq!(events[0].level, "warn");
        assert_eq!(events[0].message, "settings.saved");
        assert_eq!(
            events[0].properties["section"],
            Value::String("profile".to_owned())
        );
        assert_eq!(
            events[0].properties["windowId"],
            Value::String("main".to_owned())
        );
    }
}
