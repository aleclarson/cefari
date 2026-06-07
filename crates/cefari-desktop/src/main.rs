use std::{fs, process::ExitCode};

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, RuntimeLogConfig, RuntimePaths};
use single_instance::SingleInstance;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;

const MAIN_WINDOW_TITLE: &str = "Cefari";
const MAIN_WINDOW_WIDTH: f64 = 1200.0;
const MAIN_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_WIDTH: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 560.0;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            report_startup_error(&error);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    let guards = RuntimeGuards {
        _instance: acquire_single_instance(&paths)?,
        _log_guard: init_logging(&paths)?,
    };

    info!(config = %paths.config_file.display(), "cefari desktop startup");
    run_native_shell(guards)
}

struct RuntimeGuards {
    _instance: SingleInstance,
    _log_guard: WorkerGuard,
}

fn run_native_shell(guards: RuntimeGuards) -> Result<()> {
    let event_loop = EventLoop::new();
    let window = create_main_window(&event_loop)?;

    info!(window = ?window.id(), "cefari native shell started");
    run_event_loop(event_loop, window, guards)
}

fn create_main_window(event_loop: &EventLoop<()>) -> Result<Window> {
    WindowBuilder::new()
        .with_title(MAIN_WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT))
        .build(event_loop)
        .context("failed to create Cefari main window")
}

fn run_event_loop(event_loop: EventLoop<()>, window: Window, guards: RuntimeGuards) -> ! {
    let mut window = Some(window);

    event_loop.run(move |event, _, control_flow| {
        let _guards = &guards;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window = None;
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                if let Some(window) = &window {
                    window.request_redraw();
                }
            }
            Event::LoopDestroyed => {
                info!("cefari native shell stopped");
            }
            _ => {}
        }
    });
}

fn report_startup_error(error: &anyhow::Error) {
    error!(error = %error, "cefari desktop startup failed");
    eprintln!("{}", startup_error_message(error));
}

fn startup_error_message(error: &anyhow::Error) -> String {
    format!("Cefari failed to start before the UI was available: {error}")
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

#[cfg(test)]
mod tests {
    use super::{
        MAIN_WINDOW_HEIGHT, MAIN_WINDOW_TITLE, MAIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT,
        MIN_WINDOW_WIDTH, startup_error_message,
    };

    #[test]
    fn startup_error_message_names_pre_ui_failure() {
        let message = startup_error_message(&anyhow::anyhow!("missing resources"));

        assert!(message.contains("before the UI was available"));
        assert!(message.contains("missing resources"));
    }

    #[test]
    fn main_window_spec_is_large_enough_for_desktop_shell() {
        assert_eq!(MAIN_WINDOW_TITLE, "Cefari");
        assert!(std::hint::black_box(MAIN_WINDOW_WIDTH) >= std::hint::black_box(MIN_WINDOW_WIDTH));
        assert!(
            std::hint::black_box(MAIN_WINDOW_HEIGHT) >= std::hint::black_box(MIN_WINDOW_HEIGHT)
        );
    }
}
