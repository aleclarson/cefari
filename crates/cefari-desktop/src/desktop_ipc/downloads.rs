use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, DownloadCommand, DownloadResult};

use super::invalid_command;

pub trait DownloadContext {
    fn download(&mut self, command: &DownloadCommand) -> Result<DownloadResult>;
}

pub fn dispatch(
    command: &DownloadCommand,
    context: &mut impl DownloadContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .download(command)
        .map(CefariIpcResult::Download)
        .map_err(|error| invalid_command(&error, "download"))
}
