use std::fs;

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, RuntimeLogConfig, RuntimePaths};
use single_instance::SingleInstance;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

fn main() -> Result<()> {
    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    let _instance = acquire_single_instance(&paths)?;
    let _log_guard = init_logging(&paths)?;

    info!(config = %paths.config_file.display(), "cefari desktop startup");

    Ok(())
}

fn acquire_single_instance(paths: &RuntimePaths) -> Result<SingleInstance> {
    fs::create_dir_all(&paths.cache_dir).with_context(|| {
        format!(
            "failed to create cache directory at {}",
            paths.cache_dir.display()
        )
    })?;

    let lock_path = paths.cache_dir.join("cefari.lock");
    let instance = SingleInstance::new(&lock_path.display().to_string()).with_context(|| {
        format!(
            "failed to create single-instance lock at {}",
            lock_path.display()
        )
    })?;

    if instance.is_single() {
        Ok(instance)
    } else {
        anyhow::bail!("another Cefari instance is already running")
    }
}

fn init_logging(paths: &RuntimePaths) -> Result<WorkerGuard> {
    let log_config = RuntimeLogConfig::new(paths);
    fs::create_dir_all(&log_config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            log_config.directory.display()
        )
    })?;

    let file_appender =
        tracing_appender::rolling::never(&log_config.directory, &log_config.file_name);
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(guard)
}
