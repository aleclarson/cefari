use anyhow::Result;
use cefari_core::{
    CefariIpcEvent, DialogCommand, DialogResult, DownloadCommand, DownloadResult, FileResult,
    FilesCommand, RuntimePaths, ServiceStatusResult, TrayResult, UpdateCheckResult,
    UpdateStateResult, WindowCreateRequest, WindowIdEvent, WindowListResult, WindowSetTitleRequest,
    WindowState, WindowStateEvent, WindowTargetRequest,
};
use tracing::debug;

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

    fn window_current(&mut self) -> Result<WindowState> {
        Ok(self.window_manager.main_state())
    }

    fn window_list(&mut self) -> Result<WindowListResult> {
        Ok(WindowListResult {
            windows: vec![self.window_manager.main_state()],
        })
    }

    fn window_create(&mut self, _request: &WindowCreateRequest) -> Result<WindowState> {
        anyhow::bail!("creating secondary windows is not available yet")
    }

    fn window_show(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        ensure_main_target(request)?;
        let state = self.window_manager.show_main()?;
        self.emit_event(&CefariIpcEvent::WindowShown(state_event(&state)));
        Ok(state)
    }

    fn window_focus(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        ensure_main_target(request)?;
        let state = self.window_manager.focus_main()?;
        self.emit_event(&CefariIpcEvent::WindowFocused(state_event(&state)));
        Ok(state)
    }

    fn window_close(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        ensure_main_target(request)?;
        let state = self.window_manager.close_main();
        self.emit_event(&CefariIpcEvent::WindowClosed(WindowIdEvent {
            window_id: state.id.clone(),
        }));
        Ok(state)
    }

    fn window_set_title(&mut self, request: &WindowSetTitleRequest) -> Result<WindowState> {
        ensure_main_target(&WindowTargetRequest {
            target: request.target.clone(),
        })?;
        let state = self.window_manager.set_main_title(&request.title)?;
        self.emit_event(&CefariIpcEvent::WindowTitleChanged(state_event(&state)));
        Ok(state)
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
        self.window_focus(&WindowTargetRequest { target: None })?;
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

fn state_event(state: &WindowState) -> WindowStateEvent {
    WindowStateEvent {
        window_id: state.id.clone(),
        state: state.clone(),
    }
}

fn ensure_main_target(request: &WindowTargetRequest) -> Result<()> {
    let Some(target) = &request.target else {
        return Ok(());
    };
    let Some(id) = target.id.as_deref() else {
        return Ok(());
    };

    if id == window::MAIN_WINDOW_ID {
        Ok(())
    } else {
        anyhow::bail!("window {id} is not available")
    }
}

impl DesktopShellContext<'_> {
    fn emit_event(&self, event: &CefariIpcEvent) {
        if let Err(error) = self.cef_runtime.emit_event(event) {
            debug!(%error, ?event, "failed to emit Cefari IPC event");
        }
    }
}
