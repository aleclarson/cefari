use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, LogRequest};

use super::invalid_command;

pub trait LogContext {
    fn log(&mut self, request: &LogRequest) -> Result<()>;
}

pub fn dispatch(
    request: &LogRequest,
    context: &mut impl LogContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .log(request)
        .map(|()| CefariIpcResult::Empty)
        .map_err(|error| invalid_command(&error, "log"))
}
