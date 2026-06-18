use cefari_core::{
    CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    CefariIpcResult,
};

pub mod app;
pub mod dialogs;
pub mod downloads;
pub mod files;
pub mod notifications;
pub mod service;
pub mod shell;
pub mod tray;
pub mod updates;
pub mod windows;
pub mod workers;

pub use notifications::unsupported_notification;
pub use updates::{update_apply_result, update_check_result, update_state_result};

#[derive(Debug, Default)]
pub struct DesktopIpcDispatcher;

pub trait DesktopIpcContext:
    app::AppContext
    + windows::WindowContext
    + shell::ShellContext
    + updates::UpdateContext
    + service::ServiceContext
    + tray::TrayContext
    + downloads::DownloadContext
    + dialogs::DialogContext
    + notifications::NotificationContext
    + files::FilesContext
    + workers::WorkersContext
{
}

impl<T> DesktopIpcContext for T where
    T: app::AppContext
        + windows::WindowContext
        + shell::ShellContext
        + updates::UpdateContext
        + service::ServiceContext
        + tray::TrayContext
        + downloads::DownloadContext
        + dialogs::DialogContext
        + notifications::NotificationContext
        + files::FilesContext
        + workers::WorkersContext
{
}

impl DesktopIpcDispatcher {
    pub fn dispatch(
        request: CefariIpcRequest,
        context: &mut impl DesktopIpcContext,
    ) -> CefariIpcResponse {
        let result = dispatch_command(&request.command, context);

        CefariIpcResponse {
            id: request.id,
            outcome: match result {
                Ok(result) => CefariIpcOutcome::Ok(result),
                Err(error) => CefariIpcOutcome::Err(error),
            },
        }
    }
}

fn dispatch_command(
    command: &CefariIpcCommand,
    context: &mut impl DesktopIpcContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    match command {
        CefariIpcCommand::AppQuit => app::dispatch(context),
        CefariIpcCommand::WindowCurrent
        | CefariIpcCommand::WindowList
        | CefariIpcCommand::WindowCreate(_)
        | CefariIpcCommand::WindowShow(_)
        | CefariIpcCommand::WindowFocus(_)
        | CefariIpcCommand::WindowClose(_)
        | CefariIpcCommand::WindowSetTitle(_) => windows::dispatch(command, context),
        CefariIpcCommand::OpenLogs
        | CefariIpcCommand::ReloadUi
        | CefariIpcCommand::OpenExternalUrl(_) => shell::dispatch(command, context),
        CefariIpcCommand::UpdateState
        | CefariIpcCommand::UpdateCheck
        | CefariIpcCommand::UpdateApply(_)
        | CefariIpcCommand::UpdateRestart => updates::dispatch(command, context),
        CefariIpcCommand::ServiceStatus => service::dispatch(context),
        CefariIpcCommand::TrayRestoreWindow => tray::dispatch(context),
        CefariIpcCommand::Download(command) => downloads::dispatch(command, context),
        CefariIpcCommand::Dialog(command) => dialogs::dispatch(command, context),
        CefariIpcCommand::Notification(command) => notifications::dispatch(command, context),
        CefariIpcCommand::Files(command) => files::dispatch(command, context),
        CefariIpcCommand::Worker(command) => workers::dispatch(command, context),
    }
}

pub(super) fn invalid_command(error: &anyhow::Error, command: &str) -> CefariIpcError {
    CefariIpcError::InvalidCommand {
        message: format!("{command}: {error}"),
    }
}

pub(super) fn unsupported_command(error: &anyhow::Error, command: &str) -> CefariIpcError {
    CefariIpcError::Unsupported {
        command: command.to_owned(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use cefari_core::{
        AppDataDirInfo, CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest,
        DialogCommand, DialogRequest, DialogResult, DownloadCommand, DownloadIdRequest,
        DownloadIdResult, DownloadResult, FileResult, FilesCommand, NotificationCapabilities,
        NotificationCategory, NotificationCategoryAction, NotificationCommand,
        NotificationRegisterCategoriesRequest, NotificationRemoveDeliveredRequest,
        NotificationResult, NotificationSendRequest, OpenExternalUrlRequest, ServiceStatusResult,
        TrayResult, UpdateApplyRequest, UpdateApplyResult, UpdateCheckResult, UpdateStateKind,
        UpdateStateResult, WindowCreateRequest, WindowKind, WindowListResult,
        WindowSetTitleRequest, WindowState, WindowTargetRequest, WorkerCommand, WorkerListResult,
        WorkerIdResult, WorkerResult, WorkerSpawnRequest, WorkerSpawnResult, WorkerState,
        WorkerStatus,
    };

    use super::{
        DesktopIpcDispatcher, app, dialogs, downloads, files, notifications, service, shell, tray,
        updates, windows, workers,
    };

    #[derive(Debug)]
    struct FakeShellContext {
        calls: Vec<&'static str>,
        window_title: String,
        reload_should_fail: bool,
        notifications_available: bool,
    }

    impl Default for FakeShellContext {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                window_title: "Cefari".to_owned(),
                reload_should_fail: false,
                notifications_available: true,
            }
        }
    }

    impl app::AppContext for FakeShellContext {
        fn quit_app(&mut self) -> Result<()> {
            self.calls.push("quit");
            Ok(())
        }
    }

    impl windows::WindowContext for FakeShellContext {
        fn window_current(&mut self) -> Result<WindowState> {
            self.calls.push("window_current");
            Ok(self.window_state(true, true))
        }

        fn window_list(&mut self) -> Result<WindowListResult> {
            self.calls.push("window_list");
            Ok(WindowListResult {
                windows: vec![self.window_state(true, true)],
            })
        }

        fn window_create(&mut self, _request: &WindowCreateRequest) -> Result<WindowState> {
            self.calls.push("window_create");
            Ok(self.window_state(true, true))
        }

        fn window_show(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            self.calls.push("window_show");
            Ok(self.window_state(true, false))
        }

        fn window_focus(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            self.calls.push("window_focus");
            Ok(self.window_state(true, true))
        }

        fn window_close(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            self.calls.push("window_close");
            Ok(self.window_state(false, false))
        }

        fn window_set_title(&mut self, request: &WindowSetTitleRequest) -> Result<WindowState> {
            self.calls.push("window_set_title");
            self.window_title = request.title.clone();
            Ok(self.window_state(true, true))
        }
    }

    impl shell::ShellContext for FakeShellContext {
        fn open_logs(&mut self) -> Result<()> {
            self.calls.push("open_logs");
            Ok(())
        }

        fn reload_ui(&mut self) -> Result<()> {
            self.calls.push("reload_ui");
            if self.reload_should_fail {
                anyhow::bail!("CEF main browser is not available");
            }
            Ok(())
        }

        fn open_external_url(&mut self, _url: &str) -> Result<()> {
            self.calls.push("open_external_url");
            Ok(())
        }
    }

    impl updates::UpdateContext for FakeShellContext {
        fn update_state(&mut self) -> Result<UpdateStateResult> {
            self.calls.push("update_state");
            Ok(UpdateStateResult {
                state: UpdateStateKind::Current,
            })
        }

        fn update_check(&mut self) -> Result<UpdateCheckResult> {
            self.calls.push("update_check");
            Ok(UpdateCheckResult {
                state: UpdateStateKind::Available,
                version: Some("1.2.3".to_owned()),
                update_id: Some("1.2.3".to_owned()),
            })
        }

        fn update_apply(&mut self, _update_id: Option<&str>) -> Result<UpdateApplyResult> {
            self.calls.push("update_apply");
            Ok(UpdateApplyResult {
                state: UpdateStateKind::ReadyToRestart,
                version: Some("1.2.3".to_owned()),
                restart_required: true,
            })
        }

        fn update_restart(&mut self) -> Result<()> {
            self.calls.push("update_restart");
            Ok(())
        }
    }

    impl service::ServiceContext for FakeShellContext {
        fn service_status(&mut self) -> Result<ServiceStatusResult> {
            self.calls.push("service_status");
            Ok(ServiceStatusResult {
                status: "running".to_owned(),
            })
        }
    }

    impl tray::TrayContext for FakeShellContext {
        fn tray_restore_window(&mut self) -> Result<TrayResult> {
            self.calls.push("tray_restore_window");
            Ok(TrayResult { restored: true })
        }
    }

    impl dialogs::DialogContext for FakeShellContext {
        fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult> {
            match command {
                DialogCommand::OpenFile(_) => {
                    self.calls.push("dialog_open_file");
                    Ok(DialogResult::Canceled)
                }
                _ => anyhow::bail!("unsupported test dialog command"),
            }
        }
    }

    impl downloads::DownloadContext for FakeShellContext {
        fn download(&mut self, command: &DownloadCommand) -> Result<DownloadResult> {
            match command {
                DownloadCommand::Cancel(request) => {
                    self.calls.push("download_cancel");
                    Ok(DownloadResult::Canceled(DownloadIdResult {
                        id: request.id.clone(),
                    }))
                }
                DownloadCommand::Reveal(request) => {
                    self.calls.push("download_reveal");
                    Ok(DownloadResult::Revealed(DownloadIdResult {
                        id: request.id.clone(),
                    }))
                }
            }
        }
    }

    impl notifications::NotificationContext for FakeShellContext {
        fn notification(
            &mut self,
            command: &NotificationCommand,
        ) -> Result<NotificationResult, CefariIpcError> {
            if !self.notifications_available {
                return Err(super::unsupported_notification(
                    command,
                    "desktop notifications are not available",
                ));
            }

            match command {
                NotificationCommand::PermissionState => {
                    self.calls.push("notification_permission_state");
                    Ok(NotificationResult::PermissionState { allowed: true })
                }
                NotificationCommand::RequestPermission => {
                    self.calls.push("notification_request_permission");
                    Ok(NotificationResult::PermissionRequested { allowed: true })
                }
                NotificationCommand::Capabilities => {
                    self.calls.push("notification_capabilities");
                    Ok(NotificationResult::Capabilities(NotificationCapabilities {
                        permission_state: true,
                        permission_prompt: true,
                        subtitle: true,
                        image: true,
                        icon: true,
                        icon_round_crop: true,
                        thread_id: true,
                        categories: true,
                        action_buttons: true,
                        text_input_actions: true,
                        user_info: true,
                        xdg_category: true,
                        active_notifications: true,
                        remove_delivered: true,
                        response_events: true,
                        cold_start_activation: true,
                    }))
                }
                NotificationCommand::RegisterCategories(request) => {
                    self.calls.push("notification_register_categories");
                    Ok(NotificationResult::CategoriesRegistered {
                        count: request.categories.len().try_into().unwrap_or(u32::MAX),
                    })
                }
                NotificationCommand::Send(request) => {
                    self.calls.push("notification_send");
                    if request.title.trim().is_empty() {
                        return Err(CefariIpcError::InvalidCommand {
                            message: "notification title cannot be empty".to_owned(),
                        });
                    }
                    Ok(NotificationResult::Sent {
                        id: "n1".to_owned(),
                    })
                }
                NotificationCommand::Active => {
                    self.calls.push("notification_active");
                    Ok(NotificationResult::Active {
                        notifications: Vec::new(),
                    })
                }
                NotificationCommand::RemoveDelivered(request) => {
                    self.calls.push("notification_remove_delivered");
                    Ok(NotificationResult::Removed {
                        count: request.ids.len().try_into().unwrap_or(u32::MAX),
                    })
                }
                NotificationCommand::RemoveAllDelivered => {
                    self.calls.push("notification_remove_all_delivered");
                    Ok(NotificationResult::Removed { count: 0 })
                }
            }
        }
    }

    impl files::FilesContext for FakeShellContext {
        fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
            match command {
                FilesCommand::AppDataDir => {
                    self.calls.push("files_app_data_dir");
                    Ok(FileResult::AppDataDir(AppDataDirInfo {
                        root_kind: "appData".to_owned(),
                        display_path: "/tmp/cefari".to_owned(),
                    }))
                }
                FilesCommand::Exists(_) => {
                    self.calls.push("files_exists");
                    Ok(FileResult::Exists { exists: true })
                }
                _ => anyhow::bail!("unsupported test file command"),
            }
        }
    }

    impl workers::WorkersContext for FakeShellContext {
        fn worker(&mut self, command: &WorkerCommand) -> Result<WorkerResult, CefariIpcError> {
            match command {
                WorkerCommand::Spawn(request) => {
                    self.calls.push("worker_spawn");
                    Ok(WorkerResult::Spawned(WorkerSpawnResult {
                        id: "worker-1".to_owned(),
                        worker: request.worker.clone(),
                        status: WorkerStatus::Running,
                    }))
                }
                WorkerCommand::Terminate(request) => {
                    self.calls.push("worker_terminate");
                    Ok(WorkerResult::Terminated(WorkerIdResult {
                        id: request.id.clone(),
                    }))
                }
                WorkerCommand::List => {
                    self.calls.push("worker_list");
                    Ok(WorkerResult::List(WorkerListResult {
                        workers: vec![WorkerState {
                            id: "worker-1".to_owned(),
                            worker: "thumbnailer".to_owned(),
                            status: WorkerStatus::Running,
                        }],
                    }))
                }
            }
        }
    }

    impl FakeShellContext {
        fn window_state(&self, visible: bool, focused: bool) -> WindowState {
            WindowState {
                id: "main".to_owned(),
                kind: WindowKind::Main,
                visible,
                focused,
                title: self.window_title.clone(),
                modal: false,
                parent_id: None,
                route: None,
            }
        }
    }

    #[test]
    fn dispatches_supported_native_commands_to_context() {
        let mut context = FakeShellContext::default();
        let commands = [
            CefariIpcCommand::AppQuit,
            CefariIpcCommand::WindowCurrent,
            CefariIpcCommand::WindowList,
            CefariIpcCommand::WindowCreate(WindowCreateRequest {
                id: Some("settings".to_owned()),
                route: Some("/settings".to_owned()),
                title: Some("Settings".to_owned()),
                width: Some(720),
                height: Some(560),
                min_width: None,
                min_height: None,
                max_width: None,
                max_height: None,
                x: None,
                y: None,
                visible: None,
                focused: None,
                resizable: None,
                decorations: None,
                always_on_top: None,
                parent_id: None,
                modal: None,
                persist_key: None,
            }),
            CefariIpcCommand::WindowShow(WindowTargetRequest { target: None }),
            CefariIpcCommand::WindowFocus(WindowTargetRequest { target: None }),
            CefariIpcCommand::WindowClose(WindowTargetRequest { target: None }),
            CefariIpcCommand::WindowSetTitle(WindowSetTitleRequest {
                target: None,
                title: "New Title".to_owned(),
            }),
            CefariIpcCommand::OpenLogs,
            CefariIpcCommand::ReloadUi,
            CefariIpcCommand::OpenExternalUrl(OpenExternalUrlRequest {
                url: "https://cefari.dev".to_owned(),
            }),
            CefariIpcCommand::UpdateState,
            CefariIpcCommand::UpdateCheck,
            CefariIpcCommand::UpdateApply(UpdateApplyRequest {
                update_id: Some("1.2.3".to_owned()),
            }),
            CefariIpcCommand::UpdateRestart,
            CefariIpcCommand::ServiceStatus,
            CefariIpcCommand::TrayRestoreWindow,
            CefariIpcCommand::Download(DownloadCommand::Cancel(DownloadIdRequest {
                id: "cef-1".to_owned(),
            })),
            CefariIpcCommand::Dialog(DialogCommand::OpenFile(DialogRequest {
                title: None,
                filters: Vec::new(),
                default_directory: None,
                default_name: None,
                modality: None,
                can_create_directories: None,
            })),
            CefariIpcCommand::Notification(NotificationCommand::PermissionState),
            CefariIpcCommand::Notification(NotificationCommand::RequestPermission),
            CefariIpcCommand::Notification(NotificationCommand::Capabilities),
            CefariIpcCommand::Notification(NotificationCommand::RegisterCategories(
                NotificationRegisterCategoriesRequest {
                    categories: vec![NotificationCategory {
                        id: "message".to_owned(),
                        actions: vec![NotificationCategoryAction::Action {
                            id: "open".to_owned(),
                            title: "Open".to_owned(),
                        }],
                    }],
                },
            )),
            CefariIpcCommand::Notification(NotificationCommand::Send(NotificationSendRequest {
                title: "Done".to_owned(),
                body: None,
                subtitle: None,
                image: None,
                icon: None,
                icon_round_crop: false,
                thread_id: None,
                category_id: None,
                user_info: Default::default(),
                xdg_category: None,
            })),
            CefariIpcCommand::Notification(NotificationCommand::Active),
            CefariIpcCommand::Notification(NotificationCommand::RemoveDelivered(
                NotificationRemoveDeliveredRequest {
                    ids: vec!["n1".to_owned()],
                },
            )),
            CefariIpcCommand::Notification(NotificationCommand::RemoveAllDelivered),
            CefariIpcCommand::Files(FilesCommand::AppDataDir),
            CefariIpcCommand::Files(FilesCommand::Exists(cefari_core::FilePathRequest {
                path: "state.json".to_owned(),
            })),
            CefariIpcCommand::Worker(WorkerCommand::Spawn(WorkerSpawnRequest {
                worker: "thumbnailer".to_owned(),
                input_json: r#"{"imageId":"abc"}"#.to_owned(),
            })),
            CefariIpcCommand::Worker(WorkerCommand::List),
        ];

        for (index, command) in commands.into_iter().enumerate() {
            let response = DesktopIpcDispatcher::dispatch(
                CefariIpcRequest {
                    id: index.to_string(),
                    command,
                },
                &mut context,
            );
            assert!(matches!(response.outcome, CefariIpcOutcome::Ok(_)));
        }

        assert_eq!(
            context.calls,
            [
                "quit",
                "window_current",
                "window_list",
                "window_create",
                "window_show",
                "window_focus",
                "window_close",
                "window_set_title",
                "open_logs",
                "reload_ui",
                "open_external_url",
                "update_state",
                "update_check",
                "update_apply",
                "update_restart",
                "service_status",
                "tray_restore_window",
                "download_cancel",
                "dialog_open_file",
                "notification_permission_state",
                "notification_request_permission",
                "notification_capabilities",
                "notification_register_categories",
                "notification_send",
                "notification_active",
                "notification_remove_delivered",
                "notification_remove_all_delivered",
                "files_app_data_dir",
                "files_exists",
                "worker_spawn",
                "worker_list",
            ]
        );
    }

    #[test]
    fn returns_typed_invalid_errors_for_invalid_notification_commands() {
        let mut context = FakeShellContext::default();
        let command =
            CefariIpcCommand::Notification(NotificationCommand::Send(NotificationSendRequest {
                title: " ".to_owned(),
                body: None,
                subtitle: None,
                image: None,
                icon: None,
                icon_round_crop: false,
                thread_id: None,
                category_id: None,
                user_info: Default::default(),
                xdg_category: None,
            }));
        let response = DesktopIpcDispatcher::dispatch(
            CefariIpcRequest {
                id: "invalid-notification".to_owned(),
                command,
            },
            &mut context,
        );

        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::InvalidCommand { .. })
        ));
    }

    #[test]
    fn returns_typed_unsupported_when_notifications_are_unavailable() {
        let mut context = FakeShellContext {
            notifications_available: false,
            ..Default::default()
        };

        let response = DesktopIpcDispatcher::dispatch(
            CefariIpcRequest {
                id: "notifications-disabled".to_owned(),
                command: CefariIpcCommand::Notification(NotificationCommand::Capabilities),
            },
            &mut context,
        );

        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::Unsupported { .. })
        ));
    }

    #[test]
    fn reload_ui_returns_typed_unsupported_when_browser_is_missing() {
        let mut context = FakeShellContext {
            reload_should_fail: true,
            ..Default::default()
        };

        let response = DesktopIpcDispatcher::dispatch(
            CefariIpcRequest {
                id: "reload".to_owned(),
                command: CefariIpcCommand::ReloadUi,
            },
            &mut context,
        );

        assert_eq!(context.calls, ["reload_ui"]);
        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::Unsupported { .. })
        ));
    }
}
