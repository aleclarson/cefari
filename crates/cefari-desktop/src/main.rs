use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, RuntimeLogConfig, RuntimePaths};
use single_instance::SingleInstance;
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;

mod desktop_menu;
mod desktop_tray;
mod external;
mod runtime;

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
    let runtime_operations = runtime::RuntimeOperations::load(&paths)?;
    let update_state = runtime_operations.update_check_config();

    info!(
        config = %paths.config_file.display(),
        updates_configured = update_state.is_configured(),
        daemon = %runtime_operations.daemon_service_spec().program.display(),
        "cefari desktop startup"
    );
    run_native_shell(guards, paths)
}

struct RuntimeGuards {
    _instance: SingleInstance,
    _log_guard: WorkerGuard,
}

#[derive(Debug)]
enum UserEvent {
    Menu(muda::MenuEvent),
    Tray(tray_icon::TrayIconEvent),
}

fn run_native_shell(guards: RuntimeGuards, paths: RuntimePaths) -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let event_proxy = event_loop.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Menu(event));
    }));
    let event_proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Tray(event));
    }));

    let window = create_main_window(&event_loop)?;
    let menu = desktop_menu::DesktopMenu::new()?;
    menu.install();

    info!(window = ?window.id(), "cefari native shell started");
    run_event_loop(event_loop, window, guards, menu, paths.log_dir)
}

fn create_main_window(event_loop: &EventLoop<UserEvent>) -> Result<Window> {
    WindowBuilder::new()
        .with_title(MAIN_WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT))
        .build(event_loop)
        .context("failed to create Cefari main window")
}

fn run_event_loop(
    event_loop: EventLoop<UserEvent>,
    window: Window,
    guards: RuntimeGuards,
    menu: desktop_menu::DesktopMenu,
    logs_dir: PathBuf,
) -> ! {
    let mut window = Some(window);
    let mut tray = None;

    event_loop.run(move |event, _, control_flow| {
        let _guards = &guards;
        let _menu = &menu;
        let _tray = &tray;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => match desktop_tray::DesktopTray::new() {
                Ok(desktop_tray) => {
                    tray = Some(desktop_tray);
                }
                Err(error) => {
                    error!(%error, "failed to initialize tray icon");
                }
            },
            Event::UserEvent(UserEvent::Menu(menu_event)) => {
                match desktop_menu::handle_menu_event(&menu_event, &logs_dir) {
                    Ok(desktop_menu::MenuCommand::Quit) => {
                        window = None;
                        *control_flow = ControlFlow::Exit;
                    }
                    Ok(desktop_menu::MenuCommand::Unhandled) => {
                        debug!(id = %menu_event.id.as_ref(), "unhandled menu event");
                    }
                    Ok(_) => {}
                    Err(error) => {
                        error!(id = %menu_event.id.as_ref(), %error, "failed to handle menu event");
                    }
                }
            }
            Event::UserEvent(UserEvent::Tray(tray_event))
                if desktop_tray::handle_tray_event(&tray_event)
                    == desktop_tray::TrayAction::RestoreWindow =>
            {
                if let Some(window) = &window {
                    window.set_visible(true);
                    window.set_focus();
                }
            }
            Event::UserEvent(UserEvent::Tray(tray_event)) => {
                let _ = desktop_tray::handle_tray_event(&tray_event);
            }
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
            Event::Opened { urls } => {
                for url in urls {
                    let result = if url.scheme() == "file" {
                        url.to_file_path().map_or_else(
                            |()| {
                                Err(anyhow::anyhow!(
                                    "file URL cannot be converted to a local path: {url}"
                                ))
                            },
                            |path| external::open_external_file(&path),
                        )
                    } else {
                        external::open_external_url(url.as_str())
                    };

                    if let Err(error) = result {
                        error!(%url, %error, "failed to open external URL");
                    }
                }
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
