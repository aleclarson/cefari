use anyhow::Result;
use cefari_core::{
    CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse,
    CefariIpcResult, ExternalUrlResult, FileResult, FilesCommand, NotificationCommand,
    ServiceStatusResult, TrayResult, UpdateCheckResult, UpdateCheckState, UpdateStateKind,
    UpdateStateResult, WindowState,
};

#[derive(Debug, Default)]
pub struct DesktopIpcDispatcher;

pub trait NativeShellContext {
    fn quit_app(&mut self) -> Result<()>;
    fn window_show(&mut self) -> Result<WindowState>;
    fn window_focus(&mut self) -> Result<WindowState>;
    fn window_close(&mut self) -> Result<WindowState>;
    fn window_set_title(&mut self, title: &str) -> Result<WindowState>;
    fn open_logs(&mut self) -> Result<()>;
    fn open_external_url(&mut self, url: &str) -> Result<()>;
    fn update_state(&mut self) -> Result<UpdateStateResult>;
    fn update_check(&mut self) -> Result<UpdateCheckResult>;
    fn service_status(&mut self) -> Result<ServiceStatusResult>;
    fn tray_restore_window(&mut self) -> Result<TrayResult>;
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
        CefariIpcCommand::WindowShow => context
            .window_show()
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowShow")),
        CefariIpcCommand::WindowFocus => context
            .window_focus()
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowFocus")),
        CefariIpcCommand::WindowClose => context
            .window_close()
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowClose")),
        CefariIpcCommand::WindowSetTitle(request) => context
            .window_set_title(&request.title)
            .map(CefariIpcResult::Window)
            .map_err(|error| invalid_command(&error, "windowSetTitle")),
        CefariIpcCommand::OpenLogs => context
            .open_logs()
            .map(|()| CefariIpcResult::Empty)
            .map_err(|error| invalid_command(&error, "openLogs")),
        CefariIpcCommand::ReloadUi => Err(CefariIpcError::Unsupported {
            command: "reloadUi".to_owned(),
            reason: "CEF UI reload is not wired yet".to_owned(),
        }),
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
        CefariIpcCommand::ServiceStatus => context
            .service_status()
            .map(CefariIpcResult::ServiceStatus)
            .map_err(|error| unsupported_command(&error, "serviceStatus")),
        CefariIpcCommand::TrayRestoreWindow => context
            .tray_restore_window()
            .map(CefariIpcResult::Tray)
            .map_err(|error| invalid_command(&error, "trayRestoreWindow")),
        CefariIpcCommand::Notification(command) => Err(unsupported_notification(command)),
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
    }
}

fn update_state_kind(state: &UpdateCheckState) -> UpdateStateKind {
    match state {
        UpdateCheckState::NotConfigured => UpdateStateKind::NotConfigured,
        UpdateCheckState::Ready | UpdateCheckState::Checking | UpdateCheckState::NoUpdate => {
            UpdateStateKind::Current
        }
        UpdateCheckState::UpdateAvailable { .. } => UpdateStateKind::Available,
        UpdateCheckState::Failed { .. } => UpdateStateKind::Error,
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
        FileResult, FilesCommand, NotificationCommand, OpenExternalUrlRequest, ServiceStatusResult,
        TrayResult, UpdateCheckResult, UpdateStateKind, UpdateStateResult, WindowSetTitleRequest,
        WindowState,
    };

    use super::{DesktopIpcDispatcher, NativeShellContext};

    #[derive(Debug)]
    struct FakeShellContext {
        calls: Vec<&'static str>,
        window_title: String,
    }

    impl Default for FakeShellContext {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                window_title: "Cefari".to_owned(),
            }
        }
    }

    impl NativeShellContext for FakeShellContext {
        fn quit_app(&mut self) -> Result<()> {
            self.calls.push("quit");
            Ok(())
        }

        fn window_show(&mut self) -> Result<WindowState> {
            self.calls.push("window_show");
            Ok(self.window_state(true, false))
        }

        fn window_focus(&mut self) -> Result<WindowState> {
            self.calls.push("window_focus");
            Ok(self.window_state(true, true))
        }

        fn window_close(&mut self) -> Result<WindowState> {
            self.calls.push("window_close");
            Ok(self.window_state(false, false))
        }

        fn window_set_title(&mut self, title: &str) -> Result<WindowState> {
            self.calls.push("window_set_title");
            self.window_title = title.to_owned();
            Ok(self.window_state(true, true))
        }

        fn open_logs(&mut self) -> Result<()> {
            self.calls.push("open_logs");
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
            })
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
                visible,
                focused,
                title: self.window_title.clone(),
            }
        }
    }

    #[test]
    fn dispatches_supported_native_commands_to_context() {
        let mut context = FakeShellContext::default();
        let commands = [
            CefariIpcCommand::AppQuit,
            CefariIpcCommand::WindowShow,
            CefariIpcCommand::WindowFocus,
            CefariIpcCommand::WindowClose,
            CefariIpcCommand::WindowSetTitle(WindowSetTitleRequest {
                title: "New Title".to_owned(),
            }),
            CefariIpcCommand::OpenLogs,
            CefariIpcCommand::OpenExternalUrl(OpenExternalUrlRequest {
                url: "https://cefari.dev".to_owned(),
            }),
            CefariIpcCommand::UpdateState,
            CefariIpcCommand::UpdateCheck,
            CefariIpcCommand::ServiceStatus,
            CefariIpcCommand::TrayRestoreWindow,
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
                "window_show",
                "window_focus",
                "window_close",
                "window_set_title",
                "open_logs",
                "open_external_url",
                "update_state",
                "update_check",
                "service_status",
                "tray_restore_window",
                "files_app_data_dir",
            ]
        );
    }

    #[test]
    fn returns_typed_unsupported_errors_for_reserved_commands() {
        let mut context = FakeShellContext::default();

        for command in [
            CefariIpcCommand::ReloadUi,
            CefariIpcCommand::Notification(NotificationCommand::PermissionState),
        ] {
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
    }
}
