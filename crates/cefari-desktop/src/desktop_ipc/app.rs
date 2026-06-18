use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult};

use super::invalid_command;

pub trait AppContext {
    fn quit_app(&mut self) -> Result<()>;
}

pub fn dispatch(context: &mut impl AppContext) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .quit_app()
        .map(|()| CefariIpcResult::Empty)
        .map_err(|error| invalid_command(&error, "appQuit"))
}
