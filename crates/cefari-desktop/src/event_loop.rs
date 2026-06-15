use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::{
    CefariIpcCommand, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    OpenExternalUrlRequest, RuntimePaths,
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::Window,
};
use tracing::{debug, error, info};

use crate::{
    desktop_app::RuntimeGuards, desktop_bridge, desktop_cef, desktop_ipc, desktop_menu,
    desktop_tray, desktop_ui, external, runtime, shell_context::DesktopShellContext, window,
};

const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
const CEFARI_SMOKE_BACKGROUND_ENV: &str = "CEFARI_SMOKE_BACKGROUND";
const CEFARI_SMOKE_EXIT_AFTER_MS_ENV: &str = "CEFARI_SMOKE_EXIT_AFTER_MS";
const CEF_MESSAGE_PUMP_FALLBACK_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub(crate) enum UserEvent {
    Menu(muda::MenuEvent),
    Tray(tray_icon::TrayIconEvent),
    SmokeExit,
    BridgeIpc(desktop_cef::CefBridgeIpcRequest),
    CefMessagePump(Instant),
}

pub(crate) fn run_native_shell(
    mut guards: RuntimeGuards,
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
    shell_ui: &desktop_ui::ShellUi,
) -> Result<()> {
    let background_smoke = smoke_background_requested();
    let devtools_enabled = dev_mode_requested();
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    configure_smoke_background_event_loop(&mut event_loop, background_smoke);
    schedule_smoke_exit_if_requested(&event_loop);
    let event_proxy = event_loop.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Menu(event));
    }));
    let event_proxy = event_loop.create_proxy();
    tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = event_proxy.send_event(UserEvent::Tray(event));
    }));
    guards
        .cef_runtime
        .set_bridge_ipc_sender(Arc::new(TaoBridgeIpcSender {
            event_proxy: event_loop.create_proxy(),
        }));
    guards
        .cef_runtime
        .set_message_pump_scheduler(Arc::new(TaoMessagePumpScheduler {
            event_proxy: event_loop.create_proxy(),
        }));
    guards
        .cef_runtime
        .set_app_scheme_resource_dir(shell_ui.app_resource_dir().to_path_buf());

    let window = window::create_main_window(&event_loop, background_smoke)?;
    window::apply_ui_diagnostic_state(&window, shell_ui);
    guards
        .cef_runtime
        .create_browser(&window, &shell_ui.url())
        .context("failed to create CEF browser")?;
    let menu = desktop_menu::DesktopMenu::new(runtime_operations.app_config(), devtools_enabled)?;
    menu.install();

    info!(window = ?window.id(), "cefari native shell started");
    run_event_loop(
        event_loop,
        window,
        guards,
        menu,
        paths,
        runtime_operations,
        devtools_enabled,
    )
}

fn run_event_loop(
    event_loop: EventLoop<UserEvent>,
    window: Window,
    guards: RuntimeGuards,
    menu: desktop_menu::DesktopMenu,
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
    devtools_enabled: bool,
) -> ! {
    #![allow(clippy::too_many_lines)]

    let mut window = Some(window);
    let mut window_title = window::default_window_title();
    let mut cef_message_pump_deadline = Some(Instant::now());
    let mut tray = None;
    event_loop.run(move |event, _, control_flow| {
        let _guards = &guards;
        let _menu = &menu;
        let _tray = &tray;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init if desktop_tray::tray_enabled(&paths) => {
                    match desktop_tray::DesktopTray::new(runtime_operations.app_config(), &paths) {
                        Ok(desktop_tray) => {
                            tray = Some(desktop_tray);
                        }
                        Err(error) => {
                            error!(%error, "failed to initialize tray icon");
                        }
                    }
                }
                StartCause::ResumeTimeReached { .. } | StartCause::WaitCancelled { .. } => {
                    pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
                }
                _ => {}
            },
            Event::UserEvent(UserEvent::CefMessagePump(deadline)) => {
                cef_message_pump_deadline =
                    Some(earliest_deadline(cef_message_pump_deadline, deadline));
                if deadline <= Instant::now() {
                    pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
                }
            }
            Event::UserEvent(UserEvent::Menu(menu_event)) => {
                let menu_command = desktop_menu::command_for_event(&menu_event);
                if menu_command == desktop_menu::MenuCommand::OpenDevTools && devtools_enabled {
                    match guards.cef_runtime.open_dev_tools() {
                        Ok(()) => info!("opened CEF Chrome DevTools"),
                        Err(error) => error!(%error, "failed to open CEF Chrome DevTools"),
                    }
                } else if let Some(command) =
                    desktop_menu::ipc_command_for_menu_command(menu_command)
                {
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
        apply_cef_message_pump_control_flow(cef_message_pump_deadline.as_ref(), control_flow);
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

pub(crate) fn smoke_background_requested() -> bool {
    std::env::var(CEFARI_SMOKE_BACKGROUND_ENV).is_ok_and(|value| value == "1")
}

fn dev_mode_requested() -> bool {
    std::env::var(CEFARI_DEV_MODE_ENV).is_ok_and(|value| value == "1")
}

#[cfg(target_os = "macos")]
fn configure_smoke_background_event_loop(
    event_loop: &mut EventLoop<UserEvent>,
    background_smoke: bool,
) {
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};

    if !background_smoke {
        return;
    }

    event_loop.set_activation_policy(ActivationPolicy::Prohibited);
    event_loop.set_dock_visibility(false);
    event_loop.set_activate_ignoring_other_apps(false);
}

#[cfg(not(target_os = "macos"))]
fn configure_smoke_background_event_loop(
    _event_loop: &mut EventLoop<UserEvent>,
    _background_smoke: bool,
) {
}

fn log_cef_lifecycle_result(result: Result<()>, success_message: &'static str) {
    match result {
        Ok(()) => debug!("{success_message}"),
        Err(error) => debug!(%error, "{success_message} skipped or failed"),
    }
}

fn cef_message_pump_deadline(delay_ms: i64) -> Instant {
    let now = Instant::now();
    if delay_ms <= 0 {
        now
    } else {
        now.checked_add(Duration::from_millis(delay_ms.unsigned_abs()))
            .unwrap_or(now)
    }
}

fn earliest_deadline(current: Option<Instant>, next: Instant) -> Instant {
    current.map_or(next, |current| current.min(next))
}

fn pump_due_cef_message_loop(
    cef_runtime: &desktop_cef::CefRuntime,
    deadline: &mut Option<Instant>,
) {
    let now = Instant::now();
    if deadline.is_some_and(|deadline| deadline <= now) {
        cef_runtime.pump_message_loop();
        *deadline = Some(now + CEF_MESSAGE_PUMP_FALLBACK_INTERVAL);
    }
}

fn apply_cef_message_pump_control_flow(deadline: Option<&Instant>, control_flow: &mut ControlFlow) {
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

struct TaoBridgeIpcSender {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl desktop_cef::BridgeIpcSender for TaoBridgeIpcSender {
    fn send_bridge_ipc(&self, request: desktop_cef::CefBridgeIpcRequest) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::BridgeIpc(request))
            .map_err(|_| anyhow::anyhow!("desktop event loop is not available"))
    }
}

struct TaoMessagePumpScheduler {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl desktop_cef::MessagePumpScheduler for TaoMessagePumpScheduler {
    fn schedule_message_pump_work(&self, delay_ms: i64) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::CefMessagePump(cef_message_pump_deadline(
                delay_ms,
            )))
            .map_err(|_| anyhow::anyhow!("desktop event loop is not available"))
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

#[cfg(test)]
mod tests {
    use super::{
        apply_cef_message_pump_control_flow, cef_message_pump_deadline, earliest_deadline,
    };
    use std::time::{Duration, Instant};
    use tao::event_loop::ControlFlow;

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

        assert_eq!(earliest_deadline(Some(later), earlier), earlier);

        let mut wait = ControlFlow::Wait;
        apply_cef_message_pump_control_flow(Some(&later), &mut wait);
        assert_eq!(wait, ControlFlow::WaitUntil(later));

        let mut exit = ControlFlow::Exit;
        apply_cef_message_pump_control_flow(Some(&later), &mut exit);
        assert_eq!(exit, ControlFlow::Exit);
    }
}
