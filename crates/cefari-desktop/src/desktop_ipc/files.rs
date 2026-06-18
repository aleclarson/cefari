use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, FileResult, FilesCommand};

use super::invalid_command;

pub trait FilesContext {
    fn files(&mut self, command: &FilesCommand) -> Result<FileResult>;
}

pub fn dispatch(
    command: &FilesCommand,
    context: &mut impl FilesContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .files(command)
        .map(CefariIpcResult::File)
        .map_err(|error| invalid_command(&error, "files"))
}
