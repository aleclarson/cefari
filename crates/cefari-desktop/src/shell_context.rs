use anyhow::Result;
use cefari_core::{
    CefariIpcEvent, DialogCommand, DialogResult, DownloadCommand, DownloadResult, FileResult,
    FilesCommand, RuntimePaths, ServiceStatusResult, TrayResult, UpdateCheckResult,
    UpdateStateResult, WindowCreateRequest, WindowIdEvent, WindowListResult, WindowSetTitleRequest,
    WindowState, WindowStateEvent, WindowTarget, WindowTargetRequest,
};
use tao::event_loop::EventLoopWindowTarget;
use tracing::debug;

use crate::{
    desktop_app, desktop_cef, desktop_dialogs, desktop_files, desktop_ipc, external, runtime,
    window, window_state,
};
use crate::{desktop_ui, event_loop::UserEvent};

pub(crate) struct DesktopShellContext<'a> {
    pub(crate) window_manager: &'a mut window::WindowManager,
    pub(crate) event_loop: &'a EventLoopWindowTarget<UserEvent>,
    pub(crate) shell_ui: &'a desktop_ui::ShellUi,
    pub(crate) paths: &'a RuntimePaths,
    pub(crate) cef_runtime: &'a mut desktop_cef::CefRuntime,
    pub(crate) runtime_operations: &'a runtime::RuntimeOperations,
    pub(crate) window_state: &'a mut window_state::WindowStateStore,
    pub(crate) source_window_id: Option<String>,
    pub(crate) should_exit: bool,
}

impl desktop_ipc::NativeShellContext for DesktopShellContext<'_> {
    fn quit_app(&mut self) -> Result<()> {
        self.window_manager.close_main();
        self.should_exit = true;
        Ok(())
    }

    fn window_current(&mut self) -> Result<WindowState> {
        let id = self
            .source_window_id
            .as_deref()
            .unwrap_or(window::MAIN_WINDOW_ID);
        self.window_manager.state(id)
    }

    fn window_list(&mut self) -> Result<WindowListResult> {
        Ok(WindowListResult {
            windows: self.window_manager.states(),
        })
    }

    fn window_create(&mut self, request: &WindowCreateRequest) -> Result<WindowState> {
        let persist_key = window_state::persist_key_from_request(request.persist_key.as_deref());
        let persisted_geometry = persist_key
            .as_deref()
            .and_then(|persist_key| self.window_state.geometry(persist_key));
        let state =
            self.window_manager
                .create_secondary(self.event_loop, request, persisted_geometry)?;
        let url = window::window_url(&self.shell_ui.url(), &state.id, state.route.as_deref())?;
        if let Err(error) = self.cef_runtime.create_browser_for_window(
            &state.id,
            self.window_manager.window(&state.id)?,
            &url,
        ) {
            let _ = self.window_manager.remove_window(&state.id);
            return Err(error);
        }
        if let Some(persist_key) = self.window_manager.persist_key(&state.id) {
            let window = self.window_manager.window(&state.id)?;
            self.window_state.stage_window(&persist_key, window);
        }
        self.emit_event(&CefariIpcEvent::WindowCreated(state_event(&state)));
        Ok(state)
    }

    fn window_show(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        let id = self.target_window_id(request)?;
        let state = self.window_manager.show_window(&id)?;
        self.emit_event(&CefariIpcEvent::WindowShown(state_event(&state)));
        Ok(state)
    }

    fn window_focus(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        let id = self.target_window_id(request)?;
        let state = self.window_manager.focus_window(&id)?;
        if let Err(error) = self.cef_runtime.focus_browser_for_window(&id, true) {
            debug!(%error, window_id = %id, "failed to focus CEF browser for window");
        }
        self.emit_event(&CefariIpcEvent::WindowFocused(state_event(&state)));
        Ok(state)
    }

    fn window_close(&mut self, request: &WindowTargetRequest) -> Result<WindowState> {
        let id = self.target_window_id(request)?;
        for closing_id in self.window_manager.window_ids_closed_with(&id) {
            if let Err(error) = self
                .cef_runtime
                .close_browser_for_window(&closing_id, false)
            {
                debug!(%error, window_id = %closing_id, "failed to close CEF browser for window");
            }
        }
        let state = self.window_manager.remove_window(&id)?;
        if id == window::MAIN_WINDOW_ID {
            self.should_exit = true;
        }
        self.emit_event(&CefariIpcEvent::WindowClosed(WindowIdEvent {
            window_id: state.id.clone(),
        }));
        Ok(state)
    }

    fn window_set_title(&mut self, request: &WindowSetTitleRequest) -> Result<WindowState> {
        let id = self.target_window_id(&WindowTargetRequest {
            target: request.target.clone(),
        })?;
        let state = self.window_manager.set_window_title(&id, &request.title)?;
        self.emit_event(&CefariIpcEvent::WindowTitleChanged(state_event(&state)));
        Ok(state)
    }

    fn open_logs(&mut self) -> Result<()> {
        external::open_external_file(&self.paths.log_dir)
    }

    fn reload_ui(&mut self) -> Result<()> {
        let id = self
            .source_window_id
            .as_deref()
            .unwrap_or(window::MAIN_WINDOW_ID);
        self.cef_runtime.reload_browser_for_window(id)
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
        self.window_focus(&WindowTargetRequest {
            target: Some(WindowTarget {
                id: Some(window::MAIN_WINDOW_ID.to_owned()),
            }),
        })?;
        Ok(TrayResult { restored: true })
    }

    fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult> {
        desktop_dialogs::dispatch(
            command,
            self.paths,
            self.window_manager.window(window::MAIN_WINDOW_ID).ok(),
        )
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

impl DesktopShellContext<'_> {
    fn emit_event(&self, event: &CefariIpcEvent) {
        if let Err(error) = self.cef_runtime.emit_event(event) {
            debug!(%error, ?event, "failed to emit Cefari IPC event");
        }
    }

    fn target_window_id(&self, request: &WindowTargetRequest) -> Result<String> {
        let id = request
            .target
            .as_ref()
            .and_then(|target| target.id.as_deref())
            .or(self.source_window_id.as_deref())
            .unwrap_or(window::MAIN_WINDOW_ID);
        self.window_manager.state(id)?;
        Ok(id.to_owned())
    }
}
