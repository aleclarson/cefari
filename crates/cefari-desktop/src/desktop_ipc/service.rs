use anyhow::Result;
use cefari_core::{CefariIpcError, CefariIpcResult, ServiceStatusResult};

use super::unsupported_command;

pub trait ServiceContext {
    fn service_status(&mut self) -> Result<ServiceStatusResult>;
}

pub fn dispatch(context: &mut impl ServiceContext) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .service_status()
        .map(CefariIpcResult::ServiceStatus)
        .map_err(|error| unsupported_command(&error, "serviceStatus"))
}
