use cefari_core::{CefariIpcError, CefariIpcResult, NotificationCommand, NotificationResult};

pub trait NotificationContext {
    fn notification(
        &mut self,
        command: &NotificationCommand,
    ) -> Result<NotificationResult, CefariIpcError>;
}

pub fn dispatch(
    command: &NotificationCommand,
    context: &mut impl NotificationContext,
) -> Result<CefariIpcResult, CefariIpcError> {
    context
        .notification(command)
        .map(CefariIpcResult::Notification)
}

pub fn unsupported_notification(
    command: &NotificationCommand,
    reason: impl Into<String>,
) -> CefariIpcError {
    CefariIpcError::Unsupported {
        command: format!("notification.{}", notification_command_name(command)),
        reason: reason.into(),
    }
}

fn notification_command_name(command: &NotificationCommand) -> &'static str {
    match command {
        NotificationCommand::PermissionState => "permissionState",
        NotificationCommand::RequestPermission => "requestPermission",
        NotificationCommand::Capabilities => "capabilities",
        NotificationCommand::RegisterCategories(_) => "registerCategories",
        NotificationCommand::Send(_) => "send",
        NotificationCommand::Active => "active",
        NotificationCommand::RemoveDelivered(_) => "removeDelivered",
        NotificationCommand::RemoveAllDelivered => "removeAllDelivered",
    }
}
