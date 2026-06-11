#[cfg(feature = "cef")]
use std::sync::Arc;
use std::{
    fs,
    process::{Command, ExitCode},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::{
    AppIdentity, CefariIpcCommand, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    FileResult, FilesCommand, LogFileConfig, OpenExternalUrlRequest, RuntimeLogConfig,
    RuntimePaths, ServiceStatusResult, TrayResult, UpdateCheckResult, UpdateStateResult,
    WindowState, prune_rotated_logs,
};
use single_instance::SingleInstance;
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod desktop_bridge;
mod desktop_cef;
mod desktop_files;
mod desktop_ipc;
mod desktop_menu;
mod desktop_notifications;
mod desktop_tray;
mod desktop_ui;
mod external;
mod runtime;

const MAIN_WINDOW_TITLE: &str = "Cefari";
const MAIN_WINDOW_WIDTH: f64 = 1200.0;
const MAIN_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_WIDTH: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 560.0;
const CEFARI_SMOKE_EXIT_AFTER_MS_ENV: &str = "CEFARI_SMOKE_EXIT_AFTER_MS";

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
    let instance = acquire_single_instance(&paths)?;
    let log_guards = init_logging(&paths)?;

    #[cfg(feature = "cef")]
    let cef_runtime = desktop_cef::initialize(&paths)?;

    #[cfg(not(feature = "cef"))]
    let cef_runtime = desktop_cef::initialize(&paths);

    let runtime_operations = runtime::RuntimeOperations::load(&paths)?;
    let desktop_notifier =
        desktop_notifications::DesktopNotifier::from_app_config(runtime_operations.app_config())?;
    let notifications_app_id = desktop_notifier.app_id().to_owned();
    let update_state = runtime_operations.update_check_config();
    let daemon_program = runtime_operations.daemon_service_spec().program;
    let shell_ui = desktop_ui::ShellUi::load(&paths)?;

    let guards = RuntimeGuards {
        _instance: instance,
        _log_guards: log_guards,
        _desktop_notifier: desktop_notifier,
        cef_runtime,
    };

    info!(
        config = %paths.config_file.display(),
        updates_configured = update_state.is_configured(),
        daemon = %daemon_program.display(),
        notifications_app_id,
        ui_entry = %shell_ui.entry_path.display(),
        ui_diagnostic = shell_ui.is_diagnostic(),
        "cefari desktop startup"
    );
    run_native_shell(guards, paths, runtime_operations, &shell_ui)
}

struct RuntimeGuards {
    _instance: SingleInstance,
    _log_guards: LogGuards,
    _desktop_notifier: desktop_notifications::DesktopNotifier,
    cef_runtime: desktop_cef::CefRuntime,
}

#[derive(Debug)]
enum UserEvent {
    Menu(muda::MenuEvent),
    Tray(tray_icon::TrayIconEvent),
    SmokeExit,
    #[cfg(feature = "cef")]
    BridgeIpc(desktop_cef::CefBridgeIpcRequest),
    #[cfg(feature = "cef")]
    CefMessagePump(Instant),
}

fn run_native_shell(
    mut guards: RuntimeGuards,
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
    shell_ui: &desktop_ui::ShellUi,
) -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    schedule_smoke_exit_if_requested(&event_loop);
    let event_proxy = event_loop.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Menu(event));
    }));
    let event_proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Tray(event));
    }));
    #[cfg(feature = "cef")]
    guards
        .cef_runtime
        .set_bridge_ipc_sender(Arc::new(TaoBridgeIpcSender {
            event_proxy: event_loop.create_proxy(),
        }));
    #[cfg(feature = "cef")]
    guards
        .cef_runtime
        .set_message_pump_scheduler(Arc::new(TaoMessagePumpScheduler {
            event_proxy: event_loop.create_proxy(),
        }));
    guards
        .cef_runtime
        .set_app_scheme_resource_dir(shell_ui.app_resource_dir().to_path_buf());

    let window = create_main_window(&event_loop)?;
    apply_ui_diagnostic_state(&window, shell_ui);
    guards
        .cef_runtime
        .create_browser(&window, &shell_ui.url())
        .context("failed to create CEF browser")?;
    let menu = desktop_menu::DesktopMenu::new()?;
    menu.install();

    info!(window = ?window.id(), "cefari native shell started");
    run_event_loop(event_loop, window, guards, menu, paths, runtime_operations)
}

fn apply_ui_diagnostic_state(window: &Window, shell_ui: &desktop_ui::ShellUi) {
    if shell_ui.is_diagnostic() {
        window.set_title("Cefari - Missing UI Resources");
        error!(ui_entry = %shell_ui.entry_path.display(), "using diagnostic UI fallback");
    }
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
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
) -> ! {
    #![allow(clippy::too_many_lines)]

    let mut window = Some(window);
    let mut window_title = MAIN_WINDOW_TITLE.to_owned();
    let mut cef_message_pump_deadline = None;
    let mut tray = None;
    event_loop.run(move |event, _, control_flow| {
        let _guards = &guards;
        let _menu = &menu;
        let _tray = &tray;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init => match desktop_tray::DesktopTray::new() {
                    Ok(desktop_tray) => {
                        tray = Some(desktop_tray);
                    }
                    Err(error) => {
                        error!(%error, "failed to initialize tray icon");
                    }
                },
                StartCause::ResumeTimeReached { .. } | StartCause::WaitCancelled { .. } => {
                    pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
                }
                _ => {}
            },
            #[cfg(feature = "cef")]
            Event::UserEvent(UserEvent::CefMessagePump(deadline)) => {
                cef_message_pump_deadline = earliest_deadline(cef_message_pump_deadline, deadline);
                if deadline <= Instant::now() {
                    pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
                }
            }
            Event::UserEvent(UserEvent::Menu(menu_event)) => {
                if let Some(command) = desktop_menu::ipc_command_for_event(&menu_event) {
                    let mut context = DesktopShellContext {
                        window: &mut window,
                        window_title: &mut window_title,
                        paths: &paths,
                        cef_runtime: &guards.cef_runtime,
                        runtime_operations: &runtime_operations,
                        should_exit: false,
                    };
                    let response = desktop_ipc::DesktopIpcDispatcher::dispatch(
                        CefariIpcRequest {
                            id: menu_event.id.as_ref().to_owned(),
                            command,
                        },
                        &mut context,
                    );
                    handle_ipc_response(&response);
                    if context.should_exit {
                        *control_flow = ControlFlow::Exit;
                    }
                } else {
                    debug!(id = %menu_event.id.as_ref(), "unhandled menu event");
                }
            }
            Event::UserEvent(UserEvent::Tray(tray_event)) => {
                if let Some(command) = desktop_tray::ipc_command_for_event(&tray_event) {
                    let mut context = DesktopShellContext {
                        window: &mut window,
                        window_title: &mut window_title,
                        paths: &paths,
                        cef_runtime: &guards.cef_runtime,
                        runtime_operations: &runtime_operations,
                        should_exit: false,
                    };
                    let response = desktop_ipc::DesktopIpcDispatcher::dispatch(
                        CefariIpcRequest {
                            id: "cefari.tray.restore_window".to_owned(),
                            command,
                        },
                        &mut context,
                    );
                    handle_ipc_response(&response);
                } else {
                    desktop_tray::log_tray_event(&tray_event);
                }
            }
            Event::UserEvent(UserEvent::SmokeExit) => {
                info!("CEF live smoke requested timed desktop shutdown");
                *control_flow = ControlFlow::Exit;
            }
            #[cfg(feature = "cef")]
            Event::UserEvent(UserEvent::BridgeIpc(request)) => {
                let mut context = DesktopShellContext {
                    window: &mut window,
                    window_title: &mut window_title,
                    paths: &paths,
                    cef_runtime: &guards.cef_runtime,
                    runtime_operations: &runtime_operations,
                    should_exit: false,
                };
                let bridge = desktop_bridge::CefariBridge::new(
                    desktop_bridge::BridgeOriginPolicy::from_environment(),
                );
                let response_json = bridge.handle_json_request(
                    &request.origin,
                    &request.request_json,
                    &mut context,
                );
                if let Ok(callback) = request.callback.lock() {
                    callback.success_str(&response_json);
                }
                if context.should_exit {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Err(error) = guards.cef_runtime.close_browser(false) {
                    debug!(%error, "CEF browser close skipped or failed");
                }
                window = None;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_resized(),
                    "resized CEF browser after Tao window resize",
                );
                debug!(
                    width = size.width,
                    height = size.height,
                    "Tao window resized"
                );
            }
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    },
                ..
            } => {
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_screen_info_changed(),
                    "notified CEF browser of screen info change",
                );
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_resized(),
                    "resized CEF browser after Tao scale-factor change",
                );
                debug!(
                    scale_factor,
                    width = new_inner_size.width,
                    height = new_inner_size.height,
                    "Tao window scale factor changed"
                );
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(position),
                ..
            } => {
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_move_or_resize_started(),
                    "notified CEF browser of Tao window move",
                );
                debug!(x = position.x, y = position.y, "Tao window moved");
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } => {
                log_cef_lifecycle_result(
                    guards.cef_runtime.focus_browser(focused),
                    "updated CEF browser focus",
                );
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                if guards.cef_runtime.has_browser() {
                    log_cef_lifecycle_result(
                        guards.cef_runtime.close_browser(true),
                        "force-closed CEF browser after Tao window destruction",
                    );
                }
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
            }
            Event::LoopDestroyed => {
                info!("cefari native shell stopped");
            }
            Event::Opened { urls } => {
                for url in urls {
                    if url.scheme() == "file" {
                        url.to_file_path().map_or_else(
                            |()| {
                                error!(
                                    %url,
                                    "file URL cannot be converted to a local path: {url}"
                                );
                            },
                            |path| {
                                if let Err(error) = external::open_external_file(&path) {
                                    error!(%url, %error, "failed to open external file");
                                }
                            },
                        );
                    } else {
                        let mut context = DesktopShellContext {
                            window: &mut window,
                            window_title: &mut window_title,
                            paths: &paths,
                            cef_runtime: &guards.cef_runtime,
                            runtime_operations: &runtime_operations,
                            should_exit: false,
                        };
                        let response = desktop_ipc::DesktopIpcDispatcher::dispatch(
                            CefariIpcRequest {
                                id: "cefari.opened_url".to_owned(),
                                command: CefariIpcCommand::OpenExternalUrl(
                                    OpenExternalUrlRequest {
                                        url: url.to_string(),
                                    },
                                ),
                            },
                            &mut context,
                        );
                        handle_ipc_response(&response);
                    }
                }
            }
            _ => {}
        }
        apply_cef_message_pump_control_flow(&cef_message_pump_deadline, control_flow);
    });
}

fn schedule_smoke_exit_if_requested(event_loop: &EventLoop<UserEvent>) {
    let Some(delay) = smoke_exit_delay() else {
        return;
    };

    let event_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        thread::sleep(delay);
        let _ = event_proxy.send_event(UserEvent::SmokeExit);
    });
}

fn smoke_exit_delay() -> Option<Duration> {
    std::env::var(CEFARI_SMOKE_EXIT_AFTER_MS_ENV)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
}

fn log_cef_lifecycle_result(result: Result<()>, success_message: &'static str) {
    match result {
        Ok(()) => debug!("{success_message}"),
        Err(error) => debug!(%error, "{success_message} skipped or failed"),
    }
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
fn cef_message_pump_deadline(delay_ms: i64) -> Instant {
    let now = Instant::now();
    if delay_ms <= 0 {
        now
    } else {
        now.checked_add(Duration::from_millis(delay_ms as u64))
            .unwrap_or(now)
    }
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
fn earliest_deadline(current: Option<Instant>, next: Instant) -> Option<Instant> {
    Some(current.map_or(next, |current| current.min(next)))
}

fn pump_due_cef_message_loop(
    cef_runtime: &desktop_cef::CefRuntime,
    deadline: &mut Option<Instant>,
) {
    if deadline.is_some_and(|deadline| deadline <= Instant::now()) {
        *deadline = None;
        cef_runtime.pump_message_loop();
    }
}

fn apply_cef_message_pump_control_flow(deadline: &Option<Instant>, control_flow: &mut ControlFlow) {
    if matches!(
        *control_flow,
        ControlFlow::Exit | ControlFlow::ExitWithCode(_)
    ) {
        return;
    }

    if let Some(deadline) = deadline {
        *control_flow = if *deadline <= Instant::now() {
            ControlFlow::Poll
        } else {
            ControlFlow::WaitUntil(*deadline)
        };
    }
}

#[cfg(feature = "cef")]
struct TaoBridgeIpcSender {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

#[cfg(feature = "cef")]
impl desktop_cef::BridgeIpcSender for TaoBridgeIpcSender {
    fn send_bridge_ipc(&self, request: desktop_cef::CefBridgeIpcRequest) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::BridgeIpc(request))
            .map_err(|_| anyhow::anyhow!("desktop event loop is not available"))
    }
}

#[cfg(feature = "cef")]
struct TaoMessagePumpScheduler {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

#[cfg(feature = "cef")]
impl desktop_cef::MessagePumpScheduler for TaoMessagePumpScheduler {
    fn schedule_message_pump_work(&self, delay_ms: i64) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::CefMessagePump(cef_message_pump_deadline(
                delay_ms,
            )))
            .map_err(|_| anyhow::anyhow!("desktop event loop is not available"))
    }
}

struct DesktopShellContext<'a> {
    window: &'a mut Option<Window>,
    window_title: &'a mut String,
    paths: &'a RuntimePaths,
    cef_runtime: &'a desktop_cef::CefRuntime,
    runtime_operations: &'a runtime::RuntimeOperations,
    should_exit: bool,
}

impl desktop_ipc::NativeShellContext for DesktopShellContext<'_> {
    fn quit_app(&mut self) -> Result<()> {
        *self.window = None;
        self.should_exit = true;
        Ok(())
    }

    fn window_show(&mut self) -> Result<WindowState> {
        let window = self
            .window
            .as_ref()
            .context("main window is no longer available")?;
        window.set_visible(true);
        Ok(self.window_state(true, false))
    }

    fn window_focus(&mut self) -> Result<WindowState> {
        let window = self
            .window
            .as_ref()
            .context("main window is no longer available")?;
        window.set_visible(true);
        window.set_focus();
        Ok(self.window_state(true, true))
    }

    fn window_close(&mut self) -> Result<WindowState> {
        *self.window = None;
        Ok(self.window_state(false, false))
    }

    fn window_set_title(&mut self, title: &str) -> Result<WindowState> {
        let window = self
            .window
            .as_ref()
            .context("main window is no longer available")?;
        window.set_title(title);
        title.clone_into(self.window_title);
        Ok(self.window_state(true, true))
    }

    fn open_logs(&mut self) -> Result<()> {
        external::open_external_file(&self.paths.log_dir)
    }

    fn reload_ui(&mut self) -> Result<()> {
        self.cef_runtime.reload_browser()
    }

    fn open_external_url(&mut self, url: &str) -> Result<()> {
        external::open_external_url(url)
    }

    fn update_state(&mut self) -> Result<UpdateStateResult> {
        self.runtime_operations
            .update_state()
            .map(|state| desktop_ipc::update_state_result(&state))
    }

    fn update_check(&mut self) -> Result<UpdateCheckResult> {
        self.runtime_operations
            .update_check()
            .map(|state| desktop_ipc::update_check_result(&state))
    }

    fn update_apply(&mut self, update_id: Option<&str>) -> Result<cefari_core::UpdateApplyResult> {
        self.runtime_operations
            .apply_update(update_id)
            .map(|update| desktop_ipc::update_apply_result(&update.version))
    }

    fn update_restart(&mut self) -> Result<()> {
        restart_current_executable()?;
        *self.window = None;
        self.should_exit = true;
        Ok(())
    }

    fn service_status(&mut self) -> Result<ServiceStatusResult> {
        self.runtime_operations
            .daemon_service_status()
            .map(|status| ServiceStatusResult { status })
    }

    fn tray_restore_window(&mut self) -> Result<TrayResult> {
        self.window_focus()?;
        Ok(TrayResult { restored: true })
    }

    fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
        desktop_files::AppDataFs::open(self.paths)?.dispatch(command)
    }
}

fn restart_current_executable() -> Result<()> {
    let current_exe =
        std::env::current_exe().context("failed to resolve current executable for restart")?;
    Command::new(current_exe)
        .spawn()
        .context("failed to spawn replacement Cefari process")?;
    Ok(())
}

impl DesktopShellContext<'_> {
    fn window_state(&self, visible: bool, focused: bool) -> WindowState {
        WindowState {
            visible,
            focused,
            title: self.window_title.clone(),
        }
    }
}

fn handle_ipc_response(response: &CefariIpcResponse) {
    match &response.outcome {
        CefariIpcOutcome::Ok(result) => {
            debug!(id = %response.id, ?result, "IPC command completed");
        }
        CefariIpcOutcome::Err(error) => {
            error!(id = %response.id, ?error, "IPC command failed");
        }
    }
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

struct LogGuards {
    _app: WorkerGuard,
    _rust: WorkerGuard,
}

fn init_logging(paths: &RuntimePaths) -> Result<LogGuards> {
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

#[cfg(test)]
mod tests {
    use super::{
        MAIN_WINDOW_HEIGHT, MAIN_WINDOW_TITLE, MAIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT,
        MIN_WINDOW_WIDTH, apply_cef_message_pump_control_flow, cef_message_pump_deadline,
        earliest_deadline, startup_error_message,
    };
    use std::time::{Duration, Instant};
    use tao::event_loop::ControlFlow;

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

    #[test]
    fn cef_message_pump_deadline_handles_immediate_and_delayed_work() {
        let before = Instant::now();

        let immediate = cef_message_pump_deadline(0);
        let delayed = cef_message_pump_deadline(25);

        assert!(immediate >= before);
        assert!(delayed > immediate);
    }

    #[test]
    fn cef_message_pump_control_flow_uses_earliest_deadline_without_overriding_exit() {
        let now = Instant::now();
        let later = now + Duration::from_secs(5);
        let earlier = now + Duration::from_secs(1);

        assert_eq!(earliest_deadline(Some(later), earlier), Some(earlier));

        let mut wait = ControlFlow::Wait;
        apply_cef_message_pump_control_flow(&Some(later), &mut wait);
        assert_eq!(wait, ControlFlow::WaitUntil(later));

        let mut exit = ControlFlow::Exit;
        apply_cef_message_pump_control_flow(&Some(later), &mut exit);
        assert_eq!(exit, ControlFlow::Exit);
    }
}
