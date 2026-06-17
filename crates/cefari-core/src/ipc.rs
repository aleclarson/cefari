use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::{Type, Types};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CefariIpcRequest {
    pub id: String,
    pub command: CefariIpcCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CefariIpcResponse {
    pub id: String,
    pub outcome: CefariIpcOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
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
    UpdateApply(UpdateApplyRequest),
    UpdateRestart,
    ServiceStatus,
    TrayRestoreWindow,
    Notification(NotificationCommand),
    Files(FilesCommand),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcResult {
    Empty,
    Window(WindowState),
    ReloadUi,
    ExternalUrl(ExternalUrlResult),
    UpdateState(UpdateStateResult),
    UpdateCheck(UpdateCheckResult),
    UpdateApply(UpdateApplyResult),
    ServiceStatus(ServiceStatusResult),
    Tray(TrayResult),
    Notification(NotificationResult),
    File(FileResult),
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
    pub update_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApplyRequest {
    pub update_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApplyResult {
    pub state: UpdateStateKind,
    pub version: Option<String>,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStateKind {
    NotConfigured,
    Current,
    Checking,
    Available,
    Applying,
    ReadyToRestart,
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
#[serde(tag = "file", content = "payload", rename_all = "camelCase")]
pub enum FilesCommand {
    AppDataDir,
    ReadFile(FileReadRequest),
    WriteFile(FileWriteRequest),
    Readdir(ReadDirRequest),
    Mkdir(MkdirRequest),
    Rm(RmRequest),
    Rename(RenameRequest),
    CopyFile(CopyFileRequest),
    Stat(FilePathRequest),
    Access(FilePathRequest),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FilePathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileReadRequest {
    pub path: String,
    pub encoding: Option<FileEncoding>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteRequest {
    pub path: String,
    pub contents: FileContents,
    pub options: FileWriteOptions,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum FileContents {
    Text(String),
    Base64(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileEncoding {
    Utf8,
    Base64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteOptions {
    pub create_parents: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReadDirRequest {
    pub path: String,
    pub with_file_types: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MkdirRequest {
    pub path: String,
    pub recursive: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RmRequest {
    pub path: String,
    pub recursive: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CopyFileRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum FileResult {
    AppDataDir(AppDataDirInfo),
    Text { contents: String },
    Base64 { contents: String },
    DirEntries { entries: Vec<DirEntry> },
    Stat(FileStat),
    Access { ok: bool },
    Written(FileWriteResult),
    Empty,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppDataDirInfo {
    pub root_kind: String,
    pub display_path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub kind: FileKind,
    pub size: f64,
    pub modified_at_ms: Option<f64>,
    pub created_at_ms: Option<f64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteResult {
    pub path: String,
    pub bytes_written: f64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "notification", content = "payload", rename_all = "camelCase")]
pub enum NotificationCommand {
    PermissionState,
    RequestPermission,
    Capabilities,
    RegisterCategories(NotificationRegisterCategoriesRequest),
    Send(NotificationSendRequest),
    Active,
    RemoveDelivered(NotificationRemoveDeliveredRequest),
    RemoveAllDelivered,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSendRequest {
    pub title: String,
    pub body: Option<String>,
    pub subtitle: Option<String>,
    pub image: Option<NotificationMediaReference>,
    pub icon: Option<NotificationMediaReference>,
    pub icon_round_crop: bool,
    pub thread_id: Option<String>,
    pub category_id: Option<String>,
    pub user_info: BTreeMap<String, String>,
    pub xdg_category: Option<NotificationXdgCategory>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "source", content = "path", rename_all = "camelCase")]
pub enum NotificationMediaReference {
    AppResource(String),
    AppData(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NotificationXdgCategory {
    Call,
    CallEnded,
    CallIncoming,
    CallUnanswered,
    Device,
    DeviceAdded,
    DeviceError,
    DeviceRemoved,
    Email,
    EmailArrived,
    EmailBounced,
    Im,
    ImError,
    ImReceived,
    Network,
    NetworkConnected,
    NetworkDisconnected,
    NetworkError,
    Presence,
    PresenceOffline,
    PresenceOnline,
    Transfer,
    TransferComplete,
    TransferError,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRegisterCategoriesRequest {
    pub categories: Vec<NotificationCategory>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCategory {
    pub id: String,
    pub actions: Vec<NotificationCategoryAction>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NotificationCategoryAction {
    Action {
        id: String,
        title: String,
    },
    TextInput {
        id: String,
        title: String,
        input_button_title: String,
        input_placeholder: String,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRemoveDeliveredRequest {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCapabilities {
    pub permission_state: bool,
    pub permission_prompt: bool,
    pub subtitle: bool,
    pub image: bool,
    pub icon: bool,
    pub icon_round_crop: bool,
    pub thread_id: bool,
    pub categories: bool,
    pub action_buttons: bool,
    pub text_input_actions: bool,
    pub user_info: bool,
    pub xdg_category: bool,
    pub active_notifications: bool,
    pub remove_delivered: bool,
    pub response_events: bool,
    pub cold_start_activation: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ActiveNotification {
    pub id: String,
    pub user_info: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum NotificationResult {
    PermissionState {
        allowed: bool,
    },
    PermissionRequested {
        allowed: bool,
    },
    Capabilities(NotificationCapabilities),
    CategoriesRegistered {
        count: u32,
    },
    Sent {
        id: String,
    },
    Active {
        notifications: Vec<ActiveNotification>,
    },
    Removed {
        count: u32,
    },
    PermissionDenied,
    Unsupported {
        reason: String,
    },
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
    pub user_text: Option<String>,
    pub user_info: BTreeMap<String, String>,
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
