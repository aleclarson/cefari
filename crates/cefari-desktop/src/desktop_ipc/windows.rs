use anyhow::Result;
use cefari_core::{
    CefariIpcCommand, CefariIpcError, CefariIpcResult, WindowCreateRequest, WindowListResult,
    WindowSetTitleRequest, WindowState, WindowTargetRequest,
};

use super::{invalid_command, unsupported_command};

pub trait WindowContext {
    fn window_current(&mut self) -> Result<WindowState>;
    fn window_list(&mut self) -> Result<WindowListResult>;
    fn window_create(&mut self, request: &WindowCreateRequest) -> Result<WindowState>;
    fn window_show(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_focus(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_close(&mut self, request: &WindowTargetRequest) -> Result<WindowState>;
    fn window_set_title(&mut self, request: &WindowSetTitleRequest) -> Result<WindowState>;
}

pub fn dispatch(
    command: &CefariIpcCommand,
    context: &mut impl WindowContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    match command {
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
        _ => unreachable!("non-window command routed to window dispatcher"),
    }
}
