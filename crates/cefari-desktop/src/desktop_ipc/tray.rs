use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, TrayResult};

use super::invalid_command;

pub trait TrayContext {
    fn tray_restore_window(&mut self) -> Result<TrayResult>;
}

pub fn dispatch(context: &mut impl TrayContext) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .tray_restore_window()
        .map(CefariIpcResult::Tray)
        .map_err(|error| invalid_command(&error, "trayRestoreWindow"))
}
