use anyhow::{Context, Result};
use cefari_core::{
    CefariIpcError, FileResult, FilesCommand, NotificationCommand, NotificationResult,
    RuntimePaths, ServiceStatusResult, TrayResult, UpdateCheckResult, UpdateStateResult,
    WindowState,
};
use tao::window::Window;

use crate::{
    desktop_app, desktop_cef, desktop_files, desktop_ipc, desktop_notifications, external, runtime,
};

pub(crate) struct DesktopShellContext<'a> {
    pub(crate) window: &'a mut Option<Window>,
    pub(crate) window_title: &'a mut String,
    pub(crate) paths: &'a RuntimePaths,
    pub(crate) cef_runtime: &'a desktop_cef::CefRuntime,
    pub(crate) runtime_operations: &'a runtime::RuntimeOperations,
    pub(crate) desktop_notifier: Option<&'a desktop_notifications::DesktopNotifier>,
    pub(crate) should_exit: bool,
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
        Ok(self.window_state())
    }

    fn window_focus(&mut self) -> Result<WindowState> {
        let window = self
            .window
            .as_ref()
            .context("main window is no longer available")?;
        window.set_visible(true);
        window.set_focus();
        Ok(self.window_state())
    }

    fn window_close(&mut self) -> Result<WindowState> {
        *self.window = None;
        Ok(self.window_state())
    }

    fn window_set_title(&mut self, title: &str) -> Result<WindowState> {
        let window = self
            .window
            .as_ref()
            .context("main window is no longer available")?;
        window.set_title(title);
        title.clone_into(self.window_title);
        Ok(self.window_state())
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
        desktop_app::restart_current_executable()?;
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

    fn notification(
        &mut self,
        command: &NotificationCommand,
    ) -> Result<NotificationResult, CefariIpcError> {
        let notifier = self.desktop_notifier.ok_or_else(|| {
            desktop_ipc::unsupported_notification(
                command,
                "desktop notifications are not available",
            )
        })?;

        match command {
            NotificationCommand::PermissionState => notifier
                .permission_allowed_blocking()
                .map(|allowed| NotificationResult::PermissionState { allowed })
                .map_err(|error| desktop_ipc::unsupported_notification(command, error.to_string())),
            NotificationCommand::RequestPermission => notifier
                .request_permission_once_blocking()
                .map(|allowed| NotificationResult::PermissionRequested { allowed })
                .map_err(|error| desktop_ipc::unsupported_notification(command, error.to_string())),
            NotificationCommand::Capabilities => {
                Ok(NotificationResult::Capabilities(notifier.capabilities()))
            }
            NotificationCommand::RegisterCategories(request) => notifier
                .register_categories(&request.categories)
                .map(|count| NotificationResult::CategoriesRegistered { count })
                .map_err(|error| CefariIpcError::InvalidCommand {
                    message: format!("notification.registerCategories: {error}"),
                }),
            NotificationCommand::Send(request) => notifier
                .send_blocking(request)
                .map(|outcome| match outcome {
                    desktop_notifications::NotificationSendOutcome::Delivered { id } => {
                        NotificationResult::Sent { id }
                    }
                    desktop_notifications::NotificationSendOutcome::PermissionDenied => {
                        NotificationResult::PermissionDenied
                    }
                })
                .map_err(|error| CefariIpcError::InvalidCommand {
                    message: format!("notification.send: {error}"),
                }),
            NotificationCommand::Active => notifier
                .active_notifications_blocking()
                .map(|notifications| NotificationResult::Active { notifications })
                .map_err(|error| desktop_ipc::unsupported_notification(command, error.to_string())),
            NotificationCommand::RemoveDelivered(request) => notifier
                .remove_delivered(&request.ids)
                .map(|count| NotificationResult::Removed { count })
                .map_err(|error| desktop_ipc::unsupported_notification(command, error.to_string())),
            NotificationCommand::RemoveAllDelivered => notifier
                .remove_all_delivered_blocking()
                .map(|count| NotificationResult::Removed { count })
                .map_err(|error| desktop_ipc::unsupported_notification(command, error.to_string())),
        }
    }

    fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
        desktop_files::AppDataFs::open(self.paths)?.dispatch(command)
    }
}

impl DesktopShellContext<'_> {
    fn window_state(&self) -> WindowState {
        WindowState {
            visible: self.window.as_ref().is_some_and(Window::is_visible),
            focused: self.window.as_ref().is_some_and(Window::is_focused),
            title: self.window_title.clone(),
        }
    }
}
