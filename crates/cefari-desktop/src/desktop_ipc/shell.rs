use anyhow::Result;
use cefari_core::{CefariIpcCommand, CefariIpcError, CefariIpcResult, ExternalUrlResult};

use super::{invalid_command, unsupported_command};

pub trait ShellContext {
    fn open_logs(&mut self) -> Result<()>;
    fn reload_ui(&mut self) -> Result<()>;
    fn open_external_url(&mut self, url: &str) -> Result<()>;
}

pub fn dispatch(
    command: &CefariIpcCommand,
    context: &mut impl ShellContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    match command {
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
        _ => unreachable!("non-shell command routed to shell dispatcher"),
    }
}
