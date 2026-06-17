use anyhow::Result;
use cefari_core::{
    DialogCommand, DialogResult, DownloadCommand, DownloadResult, FileResult, FilesCommand,
    RuntimePaths, ServiceStatusResult, TrayResult, UpdateCheckResult, UpdateStateResult,
    WindowState,
};

use crate::{
    desktop_app, desktop_cef, desktop_dialogs, desktop_files, desktop_ipc, external, runtime,
    window,
};

pub(crate) struct DesktopShellContext<'a> {
    pub(crate) window_manager: &'a mut window::WindowManager,
    pub(crate) paths: &'a RuntimePaths,
    pub(crate) cef_runtime: &'a desktop_cef::CefRuntime,
    pub(crate) runtime_operations: &'a runtime::RuntimeOperations,
    pub(crate) should_exit: bool,
}

impl desktop_ipc::NativeShellContext for DesktopShellContext<'_> {
    fn quit_app(&mut self) -> Result<()> {
        self.window_manager.close_main();
        self.should_exit = true;
        Ok(())
    }

    fn window_show(&mut self) -> Result<WindowState> {
        self.window_manager.show_main()
    }

    fn window_focus(&mut self) -> Result<WindowState> {
        self.window_manager.focus_main()
    }

    fn window_close(&mut self) -> Result<WindowState> {
        Ok(self.window_manager.close_main())
    }

    fn window_set_title(&mut self, title: &str) -> Result<WindowState> {
        self.window_manager.set_main_title(title)
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
        self.window_manager.close_main();
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

    fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult> {
        desktop_dialogs::dispatch(command, self.paths, self.window_manager.main_window().ok())
    }

    fn download(&mut self, command: &DownloadCommand) -> Result<DownloadResult> {
        match command {
            DownloadCommand::Cancel(request) => self.cef_runtime.cancel_download(&request.id),
            DownloadCommand::Reveal(request) => self.cef_runtime.reveal_download(&request.id),
        }
    }

    fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
        desktop_files::AppDataFs::open(self.paths)?.dispatch(command)
    }
}
