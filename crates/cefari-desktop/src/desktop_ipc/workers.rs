use cefari_core::{CefariIpcError, CefariIpcResult, WorkerCommand, WorkerResult};

pub trait WorkersContext {
    fn worker(&mut self, command: &WorkerCommand) -> Result<WorkerResult, CefariIpcError>;
}

pub fn dispatch(
    command: &WorkerCommand,
    context: &mut impl WorkersContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context.worker(command).map(CefariIpcResult::Worker)
}
