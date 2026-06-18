use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::{
    CefariIpcCommand, CefariIpcEvent, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    DeepLinkOpenEvent, NotificationAction, NotificationEvent, NotificationResponseEvent,
    RuntimePaths, WindowCreateRequest,
};
use tao::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use tracing::{debug, error, info};

use crate::{
    desktop_app::RuntimeGuards, desktop_bridge, desktop_cef, desktop_daemon, desktop_ipc,
    desktop_menu, desktop_notifications, desktop_single_instance, desktop_tray, desktop_ui,
    external, runtime, shell_context::DesktopShellContext, window, window_state,
};

const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
const CEFARI_SMOKE_BACKGROUND_ENV: &str = "CEFARI_SMOKE_BACKGROUND";
const CEFARI_SMOKE_CREATE_WINDOW_ENV: &str = "CEFARI_SMOKE_CREATE_WINDOW";
const CEFARI_SMOKE_EXIT_AFTER_MS_ENV: &str = "CEFARI_SMOKE_EXIT_AFTER_MS";
const CEF_MESSAGE_PUMP_FALLBACK_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub(crate) enum UserEvent {
    Menu(muda::MenuEvent),
    Tray(tray_icon::TrayIconEvent),
    SmokeExit,
    BridgeIpc(desktop_cef::CefBridgeIpcRequest),
    Daemon(desktop_daemon::DaemonEvent),
    CefMessagePump(Instant),
    ForwardedDeepLink(String),
    NotificationResponse(NotificationResponseEvent),
}

pub(crate) fn run_native_shell(
    mut guards: RuntimeGuards,
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
    shell_ui: &desktop_ui::ShellUi,
    startup_deep_links: Vec<String>,
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
    if let Some(notifier) = guards.desktop_notifier.as_ref() {
        notifier
            .set_response_sink(Arc::new(TaoNotificationResponseSender {
                event_proxy: event_loop.create_proxy(),
            }))
            .context("failed to attach notification response handler")?;
    }
    guards
        .cef_runtime
        .set_app_scheme_resource_dir(shell_ui.app_resource_dir().to_path_buf());
    let deep_link_forwarder = desktop_single_instance::start_deep_link_forwarder(
        &paths,
        runtime_operations.deep_link_schemes(),
        event_loop.create_proxy(),
    )?;

    let mut window_state_store = window_state::WindowStateStore::load(&paths);
    let main_geometry = window_state_store.geometry(window_state::MAIN_WINDOW_PERSIST_KEY);
    let window = window::create_main_window(&event_loop, background_smoke, main_geometry)?;
    window_state_store.stage_window(window_state::MAIN_WINDOW_PERSIST_KEY, &window);
    window::apply_ui_diagnostic_state(&window, shell_ui);
    guards
        .cef_runtime
        .create_browser(&window, &shell_ui.url())
        .context("failed to create CEF browser")?;
    let menu = desktop_menu::DesktopMenu::new(runtime_operations.app_config(), devtools_enabled)?;
    menu.install();
    schedule_startup_deep_links(&event_loop, startup_deep_links);
    let daemon_config = if runtime_operations.daemon_configured() {
        Some(runtime_operations.daemon_process_config()?)
    } else {
        None
    };
    let daemon_event_sink = Arc::new(TaoDaemonEventSink {
        event_proxy: event_loop.create_proxy(),
    });
    let daemon_spawner = Arc::new(desktop_daemon::SystemDaemonSpawner);

    info!(window = ?window.id(), cefari_window = window::MAIN_WINDOW_ID, "cefari native shell started");
    let window_manager = window::WindowManager::with_main(window);
    run_event_loop(
        event_loop,
        window_manager,
        guards,
        deep_link_forwarder,
        menu,
        shell_ui.clone(),
        paths,
        runtime_operations,
        devtools_enabled,
        window_state_store,
        desktop_daemon::DaemonManager::new(daemon_config, daemon_spawner, daemon_event_sink),
    )
}

fn run_event_loop(
    event_loop: EventLoop<UserEvent>,
    mut window_manager: window::WindowManager,
    mut guards: RuntimeGuards,
    _deep_link_forwarder: desktop_single_instance::DeepLinkForwarder,
    menu: desktop_menu::DesktopMenu,
    shell_ui: desktop_ui::ShellUi,
    paths: RuntimePaths,
    runtime_operations: runtime::RuntimeOperations,
    devtools_enabled: bool,
    mut window_state_store: window_state::WindowStateStore,
    mut daemon_manager: desktop_daemon::DaemonManager,
) -> ! {
    #![allow(clippy::too_many_lines)]

    let mut cef_message_pump_deadline = Some(Instant::now());
    let mut tray = None;
    let mut smoke_secondary_created = false;
    event_loop.run(move |event, event_loop_target, control_flow| {
        let _menu = &menu;
        let _tray = &tray;
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(start_cause) => match start_cause {
                StartCause::Init => {
                    if desktop_tray::tray_enabled(&paths) {
                        match desktop_tray::DesktopTray::new(runtime_operations.app_config(), &paths)
                        {
                            Ok(desktop_tray) => {
                                tray = Some(desktop_tray);
                            }
                            Err(error) => {
                                error!(%error, "failed to initialize tray icon");
                            }
                        }
                    }
                    if smoke_create_window_requested() && !smoke_secondary_created {
                        smoke_secondary_created = true;
                        let mut context = DesktopShellContext {
                            window_manager: &mut window_manager,
                            event_loop: event_loop_target,
                            shell_ui: &shell_ui,
                            paths: &paths,
                            cef_runtime: &mut guards.cef_runtime,
                            runtime_operations: &runtime_operations,
                            window_state: &mut window_state_store,
                            source_window_id: None,
                            desktop_notifier: guards.desktop_notifier.as_ref(),
                            should_exit: false,
                        };
                        let response = desktop_ipc::DesktopIpcDispatcher::dispatch(
                            CefariIpcRequest {
                                id: "cefari.smoke.create_window".to_owned(),
                                command: smoke_create_window_command(),
                            },
                            &mut context,
                        );
                        handle_ipc_response(&response);
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
            Event::UserEvent(UserEvent::ForwardedDeepLink(url)) => {
                deliver_deep_link_url(&url, &mut window_manager, &guards.cef_runtime);
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
                        window_manager: &mut window_manager,
                        event_loop: event_loop_target,
                        shell_ui: &shell_ui,
                        paths: &paths,
                        cef_runtime: &mut guards.cef_runtime,
                        runtime_operations: &runtime_operations,
                        window_state: &mut window_state_store,
                        source_window_id: None,
                        desktop_notifier: guards.desktop_notifier.as_ref(),
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
                        window_manager: &mut window_manager,
                        event_loop: event_loop_target,
                        shell_ui: &shell_ui,
                        paths: &paths,
                        cef_runtime: &mut guards.cef_runtime,
                        runtime_operations: &runtime_operations,
                        window_state: &mut window_state_store,
                        source_window_id: None,
                        desktop_notifier: guards.desktop_notifier.as_ref(),
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
                let source_window_id = guards
                    .cef_runtime
                    .window_id_for_browser(request.browser_identifier)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                debug!(
                    source_window_id,
                    browser_identifier = ?request.browser_identifier,
                    "handling Cefari bridge IPC request"
                );
                let mut context = DesktopShellContext {
                    window_manager: &mut window_manager,
                    event_loop: event_loop_target,
                    shell_ui: &shell_ui,
                    paths: &paths,
                    cef_runtime: &mut guards.cef_runtime,
                    runtime_operations: &runtime_operations,
                    window_state: &mut window_state_store,
                    source_window_id: Some(source_window_id),
                    desktop_notifier: guards.desktop_notifier.as_ref(),
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
            Event::UserEvent(UserEvent::NotificationResponse(response)) => {
                handle_notification_response(
                    &guards.cef_runtime,
                    &mut window_manager,
                    response,
                );
            }
            Event::UserEvent(UserEvent::Daemon(event)) => {
                handle_daemon_event(&mut daemon_manager, event);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id: tao_window_id,
                ..
            } => {
                let window_id = window_manager
                    .window_id_for_tao(tao_window_id)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                for closing_id in window_manager.window_ids_closed_with(&window_id) {
                    if let Err(error) = guards
                        .cef_runtime
                        .close_browser_for_window(&closing_id, false)
                    {
                        debug!(%error, window_id = %closing_id, "CEF browser close skipped or failed");
                    }
                }
                let state = if window_id == window::MAIN_WINDOW_ID {
                    let state = window_manager.close_main();
                    *control_flow = ControlFlow::Exit;
                    state
                } else {
                    match window_manager.remove_window(&window_id) {
                        Ok(state) => state,
                        Err(error) => {
                            debug!(%error, %window_id, "Tao close requested for unknown Cefari window");
                            return;
                        }
                    }
                };
                emit_window_event(
                    &guards.cef_runtime,
                    &CefariIpcEvent::WindowClosed(cefari_core::WindowIdEvent {
                        window_id: state.id,
                    }),
                    "emitted Cefari window closed event",
                );
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                window_id: tao_window_id,
                ..
            } => {
                let window_id = window_manager
                    .window_id_for_tao(tao_window_id)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_resized_for_window(&window_id),
                    "resized CEF browser after Tao window resize",
                );
                debug!(
                    width = size.width,
                    height = size.height,
                    "Tao window resized"
                );
                stage_window_state(&mut window_state_store, &window_manager, &window_id);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    },
                window_id: tao_window_id,
                ..
            } => {
                let window_id = window_manager
                    .window_id_for_tao(tao_window_id)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                log_cef_lifecycle_result(
                    guards
                        .cef_runtime
                        .notify_browser_screen_info_changed_for_window(&window_id),
                    "notified CEF browser of screen info change",
                );
                log_cef_lifecycle_result(
                    guards.cef_runtime.notify_browser_resized_for_window(&window_id),
                    "resized CEF browser after Tao scale-factor change",
                );
                debug!(
                    scale_factor,
                    width = new_inner_size.width,
                    height = new_inner_size.height,
                    "Tao window scale factor changed"
                );
                stage_window_state(&mut window_state_store, &window_manager, &window_id);
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(position),
                window_id: tao_window_id,
                ..
            } => {
                let window_id = window_manager
                    .window_id_for_tao(tao_window_id)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                log_cef_lifecycle_result(
                    guards
                        .cef_runtime
                        .notify_browser_move_or_resize_started_for_window(&window_id),
                    "notified CEF browser of Tao window move",
                );
                debug!(x = position.x, y = position.y, "Tao window moved");
                stage_window_state(&mut window_state_store, &window_manager, &window_id);
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                window_id: tao_window_id,
                ..
            } => {
                let window_id = window_manager
                    .window_id_for_tao(tao_window_id)
                    .unwrap_or_else(|| window::MAIN_WINDOW_ID.to_owned());
                log_cef_lifecycle_result(
                    guards
                        .cef_runtime
                        .focus_browser_for_window(&window_id, focused),
                    "updated CEF browser focus",
                );
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                window_id: tao_window_id,
                ..
            } => {
                if let Some(window_id) = window_manager.window_id_for_tao(tao_window_id) {
                    if window_id == window::MAIN_WINDOW_ID {
                        *control_flow = ControlFlow::Exit;
                    } else if let Err(error) = window_manager.remove_window(&window_id) {
                        debug!(%error, %window_id, "Tao destroyed unknown secondary window");
                    }
                } else if guards.cef_runtime.has_browser() {
                    log_cef_lifecycle_result(
                        guards.cef_runtime.close_browser(true),
                        "force-closed CEF browser after Tao window destruction",
                    );
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                pump_due_cef_message_loop(&guards.cef_runtime, &mut cef_message_pump_deadline);
            }
            Event::LoopDestroyed => {
                if let Err(error) = window_state_store.flush() {
                    error!(%error, "failed to persist pending window state during shutdown");
                }
                info!("cefari native shell stopped");
            }
            Event::Opened { urls } => {
                for url in urls {
                    if let Some(notifier) = guards.desktop_notifier.as_ref() {
                        match notifier.activation_response_event(url.as_str()) {
                            Ok(Some(response)) => {
                                handle_notification_response(
                                    &guards.cef_runtime,
                                    &mut window_manager,
                                    response,
                                );
                                continue;
                            }
                            Ok(None) => {}
                            Err(error) => {
                                error!(%url, %error, "failed to decode notification activation URL");
                                continue;
                            }
                        }
                    }

                    match opened_url_action(url.scheme(), runtime_operations.deep_link_schemes()) {
                        OpenedUrlAction::File => {
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
                        }
                        OpenedUrlAction::DeepLink => {
                            deliver_deep_link_url(
                                url.as_str(),
                                &mut window_manager,
                                &guards.cef_runtime,
                            );
                        }
                        OpenedUrlAction::External => {
                            if let Err(error) = external::open_external_url(url.as_str()) {
                                error!(%url, %error, "failed to open external URL");
                            }
                        }
                        OpenedUrlAction::Unsupported => {
                            info!(%url, "ignored opened URL with unconfigured scheme");
                        }
                    }
                }
            }
            _ => {}
        }
        window_state_store.flush_if_due(Instant::now());
        apply_event_loop_control_flow(
            cef_message_pump_deadline,
            window_state_store.flush_deadline(),
            control_flow,
        );
    });
}

fn handle_daemon_event(
    daemon_manager: &mut desktop_daemon::DaemonManager,
    event: desktop_daemon::DaemonEvent,
) {
    match event {
        desktop_daemon::DaemonEvent::Chunk {
            connection_id,
            bytes,
        } => {
            debug!(
                ?connection_id,
                byte_count = bytes.len(),
                "received daemon stream chunk"
            );
        }
        desktop_daemon::DaemonEvent::Closed { connection_id } => {
            daemon_manager.clear_closed(connection_id);
            debug!(?connection_id, "daemon stream closed");
        }
        desktop_daemon::DaemonEvent::Error {
            connection_id,
            message,
        } => {
            daemon_manager.clear_closed(connection_id);
            error!(?connection_id, %message, "daemon stream failed");
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OpenedUrlAction {
    File,
    DeepLink,
    External,
    Unsupported,
}

fn opened_url_action(scheme: &str, deep_link_schemes: &[String]) -> OpenedUrlAction {
    if scheme.eq_ignore_ascii_case("file") {
        return OpenedUrlAction::File;
    }
    if deep_link_schemes
        .iter()
        .any(|configured| configured == scheme)
    {
        return OpenedUrlAction::DeepLink;
    }
    if matches!(scheme, "http" | "https" | "mailto") {
        return OpenedUrlAction::External;
    }
    OpenedUrlAction::Unsupported
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

fn schedule_startup_deep_links(event_loop: &EventLoop<UserEvent>, urls: Vec<String>) {
    if urls.is_empty() {
        return;
    }

    let event_proxy = event_loop.create_proxy();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        for url in urls {
            let _ = event_proxy.send_event(UserEvent::ForwardedDeepLink(url));
        }
    });
}

fn deliver_deep_link_url(
    url: &str,
    window_manager: &mut window::WindowManager,
    cef_runtime: &desktop_cef::CefRuntime,
) {
    if let Err(error) = window_manager.focus_window(window::MAIN_WINDOW_ID) {
        debug!(%error, "failed to focus main window for deep link");
    }
    let event = CefariIpcEvent::DeepLinkOpened(DeepLinkOpenEvent {
        url: url.to_owned(),
    });
    match cef_runtime.emit_event(&event) {
        Ok(()) => info!(url, "delivered opened deep link"),
        Err(error) => {
            error!(url, %error, "failed to deliver opened deep link");
        }
    }
}

fn handle_notification_response(
    cef_runtime: &desktop_cef::CefRuntime,
    window_manager: &mut window::WindowManager,
    response: NotificationResponseEvent,
) {
    let focus_window = response.action == NotificationAction::Default;
    let event = CefariIpcEvent::Notification(NotificationEvent::Response(response));
    if let Err(error) = cef_runtime.emit_ipc_event(&event) {
        error!(%error, "failed to emit notification response event");
    }
    if focus_window {
        match window_manager.show_window(window::MAIN_WINDOW_ID) {
            Ok(_) => {
                if let Err(error) = window_manager.focus_window(window::MAIN_WINDOW_ID) {
                    debug!(%error, "failed to focus main window for notification response");
                }
                if let Err(error) =
                    cef_runtime.focus_browser_for_window(window::MAIN_WINDOW_ID, true)
                {
                    debug!(%error, "failed to focus CEF browser for notification response");
                }
            }
            Err(error) => {
                debug!(%error, "notification default response could not show main window");
            }
        }
    }
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

fn smoke_create_window_requested() -> bool {
    std::env::var(CEFARI_SMOKE_CREATE_WINDOW_ENV).is_ok_and(|value| value == "1")
}

fn smoke_create_window_command() -> CefariIpcCommand {
    CefariIpcCommand::WindowCreate(WindowCreateRequest {
        id: Some("smoke-secondary".to_owned()),
        route: Some("/smoke-secondary".to_owned()),
        title: Some("Cefari Smoke Secondary".to_owned()),
        width: Some(720),
        height: Some(560),
        min_width: None,
        min_height: None,
        max_width: None,
        max_height: None,
        x: None,
        y: None,
        visible: Some(!smoke_background_requested()),
        focused: Some(!smoke_background_requested()),
        resizable: None,
        decorations: None,
        always_on_top: None,
        parent_id: None,
        modal: None,
        persist_key: None,
    })
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

fn emit_window_event(
    cef_runtime: &desktop_cef::CefRuntime,
    event: &CefariIpcEvent,
    success_message: &'static str,
) {
    log_cef_lifecycle_result(cef_runtime.emit_event(event), success_message);
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

fn stage_window_state(
    window_state_store: &mut window_state::WindowStateStore,
    window_manager: &window::WindowManager,
    window_id: &str,
) {
    let Some(persist_key) = window_manager.persist_key(window_id) else {
        return;
    };
    match window_manager.window(window_id) {
        Ok(window) => window_state_store.stage_window(&persist_key, window),
        Err(error) => debug!(%error, %window_id, "skipped window state capture"),
    }
}

fn apply_event_loop_control_flow(
    cef_deadline: Option<Instant>,
    persistence_deadline: Option<Instant>,
    control_flow: &mut ControlFlow,
) {
    if matches!(
        *control_flow,
        ControlFlow::Exit | ControlFlow::ExitWithCode(_)
    ) {
        return;
    }

    let deadline = cef_deadline.into_iter().chain(persistence_deadline).min();

    if let Some(deadline) = deadline {
        *control_flow = if deadline <= Instant::now() {
            ControlFlow::Poll
        } else {
            ControlFlow::WaitUntil(deadline)
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

#[derive(Debug)]
struct TaoDaemonEventSink {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl desktop_daemon::DaemonEventSink for TaoDaemonEventSink {
    fn send_daemon_event(&self, event: desktop_daemon::DaemonEvent) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::Daemon(event))
            .map_err(|_| anyhow::anyhow!("desktop event loop is not available"))
    }
}

#[derive(Debug)]
struct TaoNotificationResponseSender {
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl desktop_notifications::NotificationResponseSink for TaoNotificationResponseSender {
    fn send_notification_response(&self, event: NotificationResponseEvent) -> Result<()> {
        self.event_proxy
            .send_event(UserEvent::NotificationResponse(event))
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
        OpenedUrlAction, apply_event_loop_control_flow, cef_message_pump_deadline,
        earliest_deadline, opened_url_action, smoke_create_window_command,
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
    fn event_loop_control_flow_uses_earliest_deadline_without_overriding_exit() {
        let now = Instant::now();
        let later = now + Duration::from_secs(5);
        let earlier = now + Duration::from_secs(1);

        assert_eq!(earliest_deadline(Some(later), earlier), earlier);

        let mut wait = ControlFlow::Wait;
        apply_event_loop_control_flow(Some(later), None, &mut wait);
        assert_eq!(wait, ControlFlow::WaitUntil(later));

        let mut wait = ControlFlow::Wait;
        apply_event_loop_control_flow(Some(later), Some(earlier), &mut wait);
        assert_eq!(wait, ControlFlow::WaitUntil(earlier));

        let mut exit = ControlFlow::Exit;
        apply_event_loop_control_flow(Some(later), Some(earlier), &mut exit);
        assert_eq!(exit, ControlFlow::Exit);
    }

    #[test]
    fn opened_url_action_classifies_configured_deep_links() {
        let schemes = vec!["myapp".to_owned()];

        assert_eq!(opened_url_action("file", &schemes), OpenedUrlAction::File);
        assert_eq!(
            opened_url_action("myapp", &schemes),
            OpenedUrlAction::DeepLink
        );
        assert_eq!(
            opened_url_action("https", &schemes),
            OpenedUrlAction::External
        );
        assert_eq!(
            opened_url_action("unknown", &schemes),
            OpenedUrlAction::Unsupported
        );
    }

    #[test]
    fn smoke_create_window_command_targets_secondary_route() {
        let cefari_core::CefariIpcCommand::WindowCreate(request) = smoke_create_window_command()
        else {
            panic!("smoke command should create a window");
        };

        assert_eq!(request.id.as_deref(), Some("smoke-secondary"));
        assert_eq!(request.route.as_deref(), Some("/smoke-secondary"));
        assert_eq!(request.title.as_deref(), Some("Cefari Smoke Secondary"));
    }
}
