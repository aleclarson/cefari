use anyhow::Result;
use cefari_core::{
    CefariIpcCommand, CefariIpcError, CefariIpcResult, UpdateApplyResult, UpdateCheckResult,
    UpdateCheckState, UpdateStateKind, UpdateStateResult,
};

use super::unsupported_command;

pub trait UpdateContext {
    fn update_state(&mut self) -> Result<UpdateStateResult>;
    fn update_check(&mut self) -> Result<UpdateCheckResult>;
    fn update_apply(&mut self, update_id: Option<&str>) -> Result<UpdateApplyResult>;
    fn update_restart(&mut self) -> Result<()>;
}

pub fn dispatch(
    command: &CefariIpcCommand,
    context: &mut impl UpdateContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    match command {
        CefariIpcCommand::UpdateState => context
            .update_state()
            .map(CefariIpcResult::UpdateState)
            .map_err(|error| unsupported_command(&error, "updateState")),
        CefariIpcCommand::UpdateCheck => context
            .update_check()
            .map(CefariIpcResult::UpdateCheck)
            .map_err(|error| unsupported_command(&error, "updateCheck")),
        CefariIpcCommand::UpdateApply(request) => context
            .update_apply(request.update_id.as_deref())
            .map(CefariIpcResult::UpdateApply)
            .map_err(|error| unsupported_command(&error, "updateApply")),
        CefariIpcCommand::UpdateRestart => context
            .update_restart()
            .map(|()| CefariIpcResult::Empty)
            .map_err(|error| unsupported_command(&error, "updateRestart")),
        _ => unreachable!("non-update command routed to update dispatcher"),
    }
}

pub fn update_state_result(state: &UpdateCheckState) -> UpdateStateResult {
    UpdateStateResult {
        state: update_state_kind(state),
    }
}

pub fn update_check_result(state: &UpdateCheckState) -> UpdateCheckResult {
    let version = match state {
        UpdateCheckState::UpdateAvailable { version } => Some(version.clone()),
        _ => None,
    };

    UpdateCheckResult {
        state: update_state_kind(state),
        version,
        update_id: update_id_for_state(state),
    }
}

pub fn update_apply_result(version: &str) -> UpdateApplyResult {
    UpdateApplyResult {
        state: UpdateStateKind::ReadyToRestart,
        version: Some(version.to_owned()),
        restart_required: true,
    }
}

fn update_state_kind(state: &UpdateCheckState) -> UpdateStateKind {
    match state {
        UpdateCheckState::NotConfigured => UpdateStateKind::NotConfigured,
        UpdateCheckState::Ready | UpdateCheckState::NoUpdate => UpdateStateKind::Current,
        UpdateCheckState::Checking => UpdateStateKind::Checking,
        UpdateCheckState::UpdateAvailable { .. } => UpdateStateKind::Available,
        UpdateCheckState::Applying => UpdateStateKind::Applying,
        UpdateCheckState::ReadyToRestart => UpdateStateKind::ReadyToRestart,
        UpdateCheckState::Failed { .. } => UpdateStateKind::Error,
    }
}

fn update_id_for_state(state: &UpdateCheckState) -> Option<String> {
    match state {
        UpdateCheckState::UpdateAvailable { version } => Some(version.clone()),
        _ => None,
    }
}
