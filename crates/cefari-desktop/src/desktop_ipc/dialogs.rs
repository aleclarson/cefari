use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, DialogCommand, DialogResult};

use super::invalid_command;

pub trait DialogContext {
    fn dialog(&mut self, command: &DialogCommand) -> Result<DialogResult>;
}

pub fn dispatch(
    command: &DialogCommand,
    context: &mut impl DialogContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .dialog(command)
        .map(CefariIpcResult::Dialog)
        .map_err(|error| invalid_command(&error, "dialog"))
}
