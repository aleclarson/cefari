use std::fs;

use anyhow::{Context, Result};
use cefari_core::{LogFileConfig, RuntimeLogConfig, RuntimePaths, prune_rotated_logs};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub(crate) struct LogGuards {
    _app: WorkerGuard,
    _rust: WorkerGuard,
}

pub(crate) fn init_logging(paths: &RuntimePaths) -> Result<LogGuards> {
    let log_config = RuntimeLogConfig::new(paths);
    fs::create_dir_all(&log_config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            log_config.directory.display()
        )
    })?;
    prune_all_rotated_logs(&log_config);

    let (app_writer, app_guard) = log_writer(&log_config.app);
    let (rust_writer, rust_guard) = log_writer(&log_config.rust);

    let app_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(app_writer)
        .with_ansi(false);
    let rust_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(rust_writer)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(app_layer)
        .with(rust_layer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(LogGuards {
        _app: app_guard,
        _rust: rust_guard,
    })
}

fn log_writer(
    config: &LogFileConfig,
) -> (
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
) {
    let file_appender = tracing_appender::rolling::daily(&config.directory, &config.file_name);
    tracing_appender::non_blocking(file_appender)
}

fn prune_all_rotated_logs(config: &RuntimeLogConfig) {
    for stream in config.streams() {
        if let Err(error) = prune_rotated_logs(stream) {
            eprintln!(
                "failed to prune rotated {} logs in {}: {error}",
                stream.file_name,
                stream.directory.display()
            );
        }
    }
}
