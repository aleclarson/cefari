use std::process::Command;

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, RuntimePaths};
use single_instance::SingleInstance;
use tracing::{error, info};

use crate::{
    desktop_cef, desktop_notifications, desktop_single_instance, desktop_ui, event_loop, logging,
    runtime,
};

pub(crate) struct RuntimeGuards {
    pub(crate) _instance: SingleInstance,
    pub(crate) _log_guards: logging::LogGuards,
    pub(crate) desktop_notifier: Option<desktop_notifications::DesktopNotifier>,
    pub(crate) cef_runtime: desktop_cef::CefRuntime,
}

pub(crate) fn run() -> Result<()> {
    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    let runtime_operations = runtime::RuntimeOperations::load(&paths)?;
    let log_guards = logging::init_logging(
        &paths,
        runtime_operations.local_log_storage_enabled(),
        runtime_operations.sentry_log_sink_config(),
    )?;
    let instance = match desktop_single_instance::acquire_or_forward(
        &paths,
        runtime_operations.deep_link_schemes(),
        std::env::args(),
    )? {
        desktop_single_instance::InstanceStartup::Primary {
            instance,
            startup_deep_links,
        } => {
            let cef_runtime = desktop_cef::initialize(&paths, runtime_operations.browser_config())?;
            (instance, startup_deep_links, cef_runtime)
        }
        desktop_single_instance::InstanceStartup::Forwarded => {
            info!("forwarded deep link arguments to existing Cefari instance");
            return Ok(());
        }
    };
    let (instance, startup_deep_links, cef_runtime) = instance;
    let background_smoke = event_loop::smoke_background_requested();
    let desktop_notifier = if background_smoke {
        None
    } else {
        Some(desktop_notifications::DesktopNotifier::from_app_config(
            runtime_operations.app_config(),
            &paths,
        )?)
    };
    let notifications_app_id = desktop_notifier.as_ref().map_or_else(
        || "<disabled for smoke>".to_owned(),
        |notifier| notifier.app_id().to_owned(),
    );
    let update_state = runtime_operations.update_check_config();
    let daemon_program = runtime_operations
        .daemon_service_spec()
        .map(|spec| spec.program.display().to_string())
        .unwrap_or_else(|_| "<disabled>".to_owned());
    let shell_ui = desktop_ui::ShellUi::load(&paths)?;

    let guards = RuntimeGuards {
        _instance: instance,
        _log_guards: log_guards,
        desktop_notifier,
        cef_runtime,
    };

    info!(
        config = %paths.config_file.display(),
        updates_configured = update_state.is_configured(),
        daemon = %daemon_program,
        notifications_app_id,
        ui_entry = %shell_ui.entry_path.display(),
        ui_diagnostic = shell_ui.is_diagnostic(),
        "cefari desktop startup"
    );
    event_loop::run_native_shell(
        guards,
        paths,
        runtime_operations,
        &shell_ui,
        startup_deep_links,
    )
}

pub(crate) fn restart_current_executable() -> Result<()> {
    let current_exe =
        std::env::current_exe().context("failed to resolve current executable for restart")?;
    Command::new(current_exe)
        .spawn()
        .context("failed to spawn replacement Cefari process")?;
    Ok(())
}

pub(crate) fn report_startup_error(error: &anyhow::Error) {
    error!(error = %error, "cefari desktop startup failed");
    eprintln!("{}", startup_error_message(error));
}

fn startup_error_message(error: &anyhow::Error) -> String {
    format!("Cefari failed to start before the UI was available: {error}")
}

#[cfg(test)]
mod tests {
    use super::startup_error_message;

    #[test]
    fn startup_error_message_names_pre_ui_failure() {
        let message = startup_error_message(&anyhow::anyhow!("missing resources"));

        assert!(message.contains("before the UI was available"));
        assert!(message.contains("missing resources"));
    }
}
