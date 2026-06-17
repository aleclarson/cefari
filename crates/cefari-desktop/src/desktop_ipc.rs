use anyhow::Result;
use cefari_core::{
    CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    CefariIpcResult, DialogCommand, DialogResult, DownloadCommand, DownloadResult,
    ExternalUrlResult, FileResult, FilesCommand, NotificationCommand, ServiceStatusResult,
    TrayResult, UpdateApplyResult, UpdateCheckResult, UpdateCheckState, UpdateStateKind,
    UpdateStateResult, WindowCreateRequest, WindowListResult, WindowSetTitleRequest, WindowState,
    WindowTargetRequest,
};

#[derive(Debug, Default)]
pub struct DesktopIpcDispatcher;

pub trait NativeShellContext {
    fn quit_app(&mut self) -> Result<()>;
    fn window_current(&mut self) -> Result<WindowState>;
    fn window_list(&mut self) -> Result<WindowListResult>;
    fn window_create(&mut self, request: &WindowCreateRequest) -> Result<WindowState>;
    fn window_show(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_focus(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_close(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_set_title(&mut self, request: &WindowSetTitleRequest) -> Result<WindowState>;
    fn open_logs(&mut self) -> Result<()>;
    fn reload_ui(&mut self) -> Result<()>;
    fn open_external_url(&mut self, url: &str) -> Result<()>;
    fn update_state(&mut self) -> Result<UpdateStateResult>;
    fn update_check(&mut self) -> Result<UpdateCheckResult>;
    fn update_apply(&mut self, update_id: Option<&str>) -> Result<UpdateApplyResult>;
    fn update_restart(&mut self) -> Result<()>;
    fn service_status(&mut self) -> Result<ServiceStatusResult>;
    fn tray_restore_window(&mut self) -> Result<TrayResult>;
    fn download(&mut self, command: &DownloadCommand) -> Result<DownloadResult>;
    fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult>;
    fn files(&mut self, command: &FilesCommand) -> Result<FileResult>;
}

impl DesktopIpcDispatcher {
    pub fn dispatch(
        request: CefariIpcRequest,
        context: &mut impl NativeShellContext,
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
    context: &mut impl NativeShellContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    match command {
        CefariIpcCommand::AppQuit => context
            .quit_app()
            .map(|()| CefariIpcResult::Empty)
            .map_err(|error| invalid_command(&error, "appQuit")),
        CefariIpcCommand::WindowCurrent => context
            .window_current()
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowCurrent")),
        CefariIpcCommand::WindowList => context
            .window_list()
            .map(CefariIpcResult::WindowList)
            .map_err(|error| invalid_command(&error, "windowList")),
        CefariIpcCommand::WindowCreate(request) => context
            .window_create(request)
            .map(CefariIpcResult::Window)
            .map_err(|error| unsupported_command(&error, "windowCreate")),
        CefariIpcCommand::WindowShow(request) => context
            .window_show(request)
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowShow")),
        CefariIpcCommand::WindowFocus(request) => context
            .window_focus(request)
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowFocus")),
        CefariIpcCommand::WindowClose(request) => context
            .window_close(request)
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowClose")),
        CefariIpcCommand::WindowSetTitle(request) => context
            .window_set_title(request)
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowSetTitle")),
        CefariIpcCommand::OpenLogs => context
            .open_logs()
            .map(|()| CefariIpcResult::Empty)
            .map_err(|error| invalid_command(&error, "openLogs")),
        CefariIpcCommand::ReloadUi => context
            .reload_ui()
            .map(|()| CefariIpcResult::ReloadUi)
            .map_err(|error| unsupported_command(&error, "reloadUi")),
        CefariIpcCommand::OpenExternalUrl(request) => context
            .open_external_url(&request.url)
            .map(|()| {
                CefariIpcResult::ExternalUrl(ExternalUrlResult {
                    url: request.url.clone(),
                })
            })
            .map_err(|error| invalid_command(&error, "openExternalUrl")),
        CefariIpcCommand::UpdateState => context
            .update_state()
            .map(CefariIpcResult::UpdateState)
            .map_err(|error| unsupported_command(&error, "updateState")),
        CefariIpcCommand::UpdateCheck => context
            .update_check()
            .map(CefariIpcResult::UpdateCheck)
            .map_err(|error| unsupported_command(&error, "updateCheck")),
        CefariIpcCommand::UpdateApply(request) => context
            .update_apply(request.update_id.as_deref())
            .map(CefariIpcResult::UpdateApply)
            .map_err(|error| unsupported_command(&error, "updateApply")),
        CefariIpcCommand::UpdateRestart => context
            .update_restart()
            .map(|()| CefariIpcResult::Empty)
            .map_err(|error| unsupported_command(&error, "updateRestart")),
        CefariIpcCommand::ServiceStatus => context
            .service_status()
            .map(CefariIpcResult::ServiceStatus)
            .map_err(|error| unsupported_command(&error, "serviceStatus")),
        CefariIpcCommand::TrayRestoreWindow => context
            .tray_restore_window()
            .map(CefariIpcResult::Tray)
            .map_err(|error| invalid_command(&error, "trayRestoreWindow")),
        CefariIpcCommand::Download(command) => context
            .download(command)
            .map(CefariIpcResult::Download)
            .map_err(|error| invalid_command(&error, "download")),
        CefariIpcCommand::Notification(command) => Err(unsupported_notification(command)),
        CefariIpcCommand::Dialog(command) => context
            .dialog(command)
            .map(CefariIpcResult::Dialog)
            .map_err(|error| invalid_command(&error, "dialog")),
        CefariIpcCommand::Files(command) => context
            .files(command)
            .map(CefariIpcResult::File)
            .map_err(|error| invalid_command(&error, "files")),
    }
}

pub fn update_state_result(state: &UpdateCheckState) -> UpdateStateResult {
    UpdateStateResult {
        state: update_state_kind(state),
    }
}

pub fn update_check_result(state: &UpdateCheckState) -> UpdateCheckResult {
    let version = match state {
        UpdateCheckState::UpdateAvailable { version } => Some(version.clone()),
        _ => None,
    };

    UpdateCheckResult {
        state: update_state_kind(state),
        version,
        update_id: update_id_for_state(state),
    }
}

pub fn update_apply_result(version: &str) -> UpdateApplyResult {
    UpdateApplyResult {
        state: UpdateStateKind::ReadyToRestart,
        version: Some(version.to_owned()),
        restart_required: true,
    }
}

fn update_state_kind(state: &UpdateCheckState) -> UpdateStateKind {
    match state {
        UpdateCheckState::NotConfigured => UpdateStateKind::NotConfigured,
        UpdateCheckState::Ready | UpdateCheckState::NoUpdate => UpdateStateKind::Current,
        UpdateCheckState::Checking => UpdateStateKind::Checking,
        UpdateCheckState::UpdateAvailable { .. } => UpdateStateKind::Available,
        UpdateCheckState::Applying => UpdateStateKind::Applying,
        UpdateCheckState::ReadyToRestart => UpdateStateKind::ReadyToRestart,
        UpdateCheckState::Failed { .. } => UpdateStateKind::Error,
    }
}

fn update_id_for_state(state: &UpdateCheckState) -> Option<String> {
    match state {
        UpdateCheckState::UpdateAvailable { version } => Some(version.clone()),
        _ => None,
    }
}

fn invalid_command(error: &anyhow::Error, command: &str) -> CefariIpcError {
    CefariIpcError::InvalidCommand {
        message: format!("{command}: {error}"),
    }
}

fn unsupported_command(error: &anyhow::Error, command: &str) -> CefariIpcError {
    CefariIpcError::Unsupported {
        command: command.to_owned(),
        reason: error.to_string(),
    }
}

fn unsupported_notification(command: &NotificationCommand) -> CefariIpcError {
    CefariIpcError::Unsupported {
        command: format!("notification.{command:?}"),
        reason: match command {
            NotificationCommand::PermissionState => {
                "notification permission state is not exposed through IPC yet"
            }
            NotificationCommand::RequestPermission => {
                "notification permission prompts are not exposed through IPC yet"
            }
            NotificationCommand::Send(_) => "notification sending is not exposed through IPC yet",
        }
        .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use cefari_core::{
        AppDataDirInfo, CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest,
        DialogCommand, DialogRequest, DialogResult, DownloadCommand, DownloadIdRequest,
        DownloadIdResult, DownloadResult, FileResult, FilesCommand, NotificationCommand,
        OpenExternalUrlRequest, ServiceStatusResult, TrayResult, UpdateApplyRequest,
        UpdateApplyResult, UpdateCheckResult, UpdateStateKind, UpdateStateResult,
        WindowCreateRequest, WindowKind, WindowListResult, WindowSetTitleRequest, WindowState,
        WindowTargetRequest,
    };

    use super::{DesktopIpcDispatcher, NativeShellContext};

    #[derive(Debug)]
    struct FakeShellContext {
        calls: Vec<&'static str>,
        window_title: String,
        reload_should_fail: bool,
    }

    impl Default for FakeShellContext {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                window_title: "Cefari".to_owned(),
                reload_should_fail: false,
            }
        }
    }

    impl NativeShellContext for FakeShellContext {
        fn quit_app(&mut self) -> Result<()> {
            self.calls.push("quit");
            Ok(())
        }

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

        fn service_status(&mut self) -> Result<ServiceStatusResult> {
            self.calls.push("service_status");
            Ok(ServiceStatusResult {
                status: "running".to_owned(),
            })
        }

        fn tray_restore_window(&mut self) -> Result<TrayResult> {
            self.calls.push("tray_restore_window");
            Ok(TrayResult { restored: true })
        }

        fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult> {
            match command {
                DialogCommand::OpenFile(_) => {
                    self.calls.push("dialog_open_file");
                    Ok(DialogResult::Canceled)
                }
                _ => anyhow::bail!("unsupported test dialog command"),
            }
        }

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

        fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
            match command {
                FilesCommand::AppDataDir => {
                    self.calls.push("files_app_data_dir");
                    Ok(FileResult::AppDataDir(AppDataDirInfo {
                        root_kind: "appData".to_owned(),
                        display_path: "/tmp/cefari".to_owned(),
                    }))
                }
                _ => anyhow::bail!("unsupported test file command"),
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
            CefariIpcCommand::Files(FilesCommand::AppDataDir),
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
                "files_app_data_dir",
            ]
        );
    }

    #[test]
    fn returns_typed_unsupported_errors_for_reserved_commands() {
        let mut context = FakeShellContext::default();
        let command = CefariIpcCommand::Notification(NotificationCommand::PermissionState);
        let response = DesktopIpcDispatcher::dispatch(
            CefariIpcRequest {
                id: "reserved".to_owned(),
                command,
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
