use serde::{Deserialize, Serialize};
use specta::{Type, Types};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CefariIpcRequest {
    pub id: String,
    pub command: CefariIpcCommand,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CefariIpcResponse {
    pub id: String,
    pub outcome: CefariIpcOutcome,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcOutcome {
    Ok(CefariIpcResult),
    Err(CefariIpcError),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "command", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcCommand {
    AppQuit,
    WindowShow,
    WindowFocus,
    WindowClose,
    WindowSetTitle(WindowSetTitleRequest),
    OpenLogs,
    ReloadUi,
    OpenExternalUrl(OpenExternalUrlRequest),
    UpdateState,
    UpdateCheck,
    ServiceStatus,
    TrayRestoreWindow,
    Notification(NotificationCommand),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcResult {
    Empty,
    Window(WindowState),
    ReloadUi,
    ExternalUrl(ExternalUrlResult),
    UpdateState(UpdateStateResult),
    UpdateCheck(UpdateCheckResult),
    ServiceStatus(ServiceStatusResult),
    Tray(TrayResult),
    Notification(NotificationResult),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "event", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcEvent {
    WindowShown(WindowState),
    WindowFocused(WindowState),
    WindowClosed,
    TrayRestoreWindow,
    UpdateStateChanged(UpdateStateResult),
    ServiceStatusChanged(ServiceStatusResult),
    Notification(NotificationEvent),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowSetTitleRequest {
    pub title: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub visible: bool,
    pub focused: bool,
    pub title: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenExternalUrlRequest {
    pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalUrlResult {
    pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStateResult {
    pub state: UpdateStateKind,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub state: UpdateStateKind,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStateKind {
    NotConfigured,
    Current,
    Available,
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatusResult {
    pub status: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayResult {
    pub restored: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "notification", content = "payload", rename_all = "camelCase")]
pub enum NotificationCommand {
    PermissionState,
    RequestPermission,
    Send(NotificationSendRequest),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSendRequest {
    pub title: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum NotificationResult {
    PermissionState { allowed: bool },
    PermissionRequested { allowed: bool },
    Sent { id: String },
    PermissionDenied,
    Unsupported { reason: String },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "event", content = "payload", rename_all = "camelCase")]
pub enum NotificationEvent {
    Response(NotificationResponseEvent),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponseEvent {
    pub id: String,
    pub action: NotificationAction,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NotificationAction {
    Default,
    Dismiss,
    Other(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "code", content = "details", rename_all = "camelCase")]
pub enum CefariIpcError {
    InvalidCommand { message: String },
    Denied { message: String },
    UnknownCommand { command: String },
    Unsupported { command: String, reason: String },
}

#[must_use]
pub fn ipc_types() -> Types {
    Types::default()
        .register::<CefariIpcRequest>()
        .register::<CefariIpcResponse>()
        .register::<CefariIpcEvent>()
}

#[cfg(test)]
mod tests {
    use super::{
        CefariIpcCommand, CefariIpcError, CefariIpcEvent, CefariIpcOutcome, CefariIpcRequest,
        CefariIpcResponse, NotificationCommand, OpenExternalUrlRequest, WindowSetTitleRequest,
        ipc_types,
    };

    #[test]
    fn serializes_command_payloads_with_stable_tags() {
        let request = CefariIpcRequest {
            id: "request-1".to_owned(),
            command: CefariIpcCommand::WindowSetTitle(WindowSetTitleRequest {
                title: "Dashboard".to_owned(),
            }),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");

        assert_eq!(
            json,
            r#"{"id":"request-1","command":{"command":"windowSetTitle","payload":{"title":"Dashboard"}}}"#
        );
        assert_eq!(
            serde_json::from_str::<CefariIpcRequest>(&json).expect("request should deserialize"),
            request
        );
    }

    #[test]
    fn round_trips_reserved_notification_commands() {
        let request = CefariIpcRequest {
            id: "request-2".to_owned(),
            command: CefariIpcCommand::Notification(NotificationCommand::PermissionState),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcRequest>(&json).expect("request should deserialize"),
            request
        );
        assert!(json.contains("permissionState"));
    }

    #[test]
    fn round_trips_typed_error_responses() {
        let response = CefariIpcResponse {
            id: "request-3".to_owned(),
            outcome: CefariIpcOutcome::Err(CefariIpcError::Unsupported {
                command: "notification.send".to_owned(),
                reason: "not wired to the dispatcher yet".to_owned(),
            }),
        };

        let json = serde_json::to_string(&response).expect("response should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcResponse>(&json).expect("response should deserialize"),
            response
        );
        assert!(json.contains("unsupported"));
    }

    #[test]
    fn round_trips_validated_external_url_commands() {
        let request = CefariIpcRequest {
            id: "request-4".to_owned(),
            command: CefariIpcCommand::OpenExternalUrl(OpenExternalUrlRequest {
                url: "https://cefari.dev".to_owned(),
            }),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcRequest>(&json).expect("request should deserialize"),
            request
        );
        assert!(json.contains("openExternalUrl"));
    }

    #[test]
    fn exports_typescript_bindings() {
        let output = specta_typescript::Typescript::default()
            .export(&ipc_types(), specta_serde::Format)
            .expect("TypeScript export should succeed");

        assert!(output.contains("export type CefariIpcRequest"));
        assert!(output.contains("windowSetTitle"));
        assert!(output.contains("unknownCommand"));
        assert!(output.contains("permissionState"));
        assert!(output.contains("export type CefariIpcEvent"));
    }

    #[test]
    fn generated_typescript_bindings_are_current() {
        let output = specta_typescript::Typescript::default()
            .export(&ipc_types(), specta_serde::Format)
            .expect("TypeScript export should succeed");
        let checked_in = include_str!("../bindings/ipc.ts");

        assert_eq!(output, checked_in);
    }

    #[test]
    fn event_types_round_trip() {
        let event = CefariIpcEvent::WindowClosed;
        let json = serde_json::to_string(&event).expect("event should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcEvent>(&json).expect("event should deserialize"),
            event
        );
    }
}
