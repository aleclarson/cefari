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

pub fn unsupported_worker(command: &WorkerCommand, reason: impl Into<String>) -> CefariIpcError {
    CefariIpcError::Unsupported {
        command: worker_command_name(command),
        reason: reason.into(),
    }
}

fn worker_command_name(command: &WorkerCommand) -> String {
    match command {
        WorkerCommand::Spawn(_) => "worker.spawn",
        WorkerCommand::Terminate(_) => "worker.terminate",
        WorkerCommand::List => "worker.list",
    }
    .to_owned()
}
