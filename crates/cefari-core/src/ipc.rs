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
    Download(DownloadCommand),
    Notification(NotificationCommand),
    Dialog(DialogCommand),
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
    Download(DownloadResult),
    Notification(NotificationResult),
    Dialog(DialogResult),
    File(FileResult),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "event", content = "payload", rename_all = "camelCase")]
pub enum CefariIpcEvent {
    WindowShown(WindowState),
    WindowFocused(WindowState),
    WindowClosed,
    DeepLinkOpened(DeepLinkOpenEvent),
    TrayRestoreWindow,
    UpdateStateChanged(UpdateStateResult),
    ServiceStatusChanged(ServiceStatusResult),
    Download(DownloadEvent),
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
pub struct DeepLinkOpenEvent {
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
#[serde(tag = "download", content = "payload", rename_all = "camelCase")]
pub enum DownloadCommand {
    Cancel(DownloadIdRequest),
    Reveal(DownloadIdRequest),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadIdRequest {
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum DownloadResult {
    Canceled(DownloadIdResult),
    Revealed(DownloadIdResult),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadIdResult {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "event", content = "payload", rename_all = "camelCase")]
pub enum DownloadEvent {
    Started(DownloadStartedEvent),
    Progress(DownloadProgressEvent),
    Completed(DownloadCompletedEvent),
    Canceled(DownloadCanceledEvent),
    Failed(DownloadFailedEvent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStartedEvent {
    pub id: String,
    pub url: String,
    pub suggested_name: String,
    pub destination_path: Option<String>,
    pub total_bytes: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub id: String,
    pub received_bytes: f64,
    pub total_bytes: Option<f64>,
    pub percent_complete: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCompletedEvent {
    pub id: String,
    pub url: String,
    pub destination_path: String,
    pub received_bytes: f64,
    pub total_bytes: Option<f64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCanceledEvent {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFailedEvent {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "dialog", content = "payload", rename_all = "camelCase")]
pub enum DialogCommand {
    OpenFile(DialogRequest),
    OpenFiles(DialogRequest),
    ChooseFolder(DialogRequest),
    ChooseFolders(DialogRequest),
    SaveFile(DialogRequest),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DialogRequest {
    pub title: Option<String>,
    pub filters: Vec<DialogFilter>,
    pub default_directory: Option<DialogDefaultDirectory>,
    pub default_name: Option<String>,
    pub modality: Option<DialogModality>,
    pub can_create_directories: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DialogDefaultDirectory {
    AppData { path: Option<String> },
    Native { path: String },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DialogModality {
    Window,
    App,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum DialogResult {
    Canceled,
    Selected { paths: Vec<DialogSelectedPath> },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DialogSelectedPath {
    pub path: String,
    pub name: String,
    pub kind: FileKind,
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
        CefariIpcResponse, DeepLinkOpenEvent, DialogCommand, DialogDefaultDirectory, DialogFilter,
        DialogModality, DialogRequest, DialogResult, DialogSelectedPath, DownloadCommand,
        DownloadCompletedEvent, DownloadEvent, DownloadIdRequest, FileKind, NotificationCommand,
        OpenExternalUrlRequest, WindowSetTitleRequest, ipc_types,
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
    fn round_trips_dialog_commands() {
        let dialog_request = DialogRequest {
            title: Some("Choose Project".to_owned()),
            filters: vec![DialogFilter {
                name: "Images".to_owned(),
                extensions: vec!["png".to_owned(), "jpg".to_owned()],
            }],
            default_directory: Some(DialogDefaultDirectory::AppData {
                path: Some("exports".to_owned()),
            }),
            default_name: Some("report.png".to_owned()),
            modality: Some(DialogModality::Window),
            can_create_directories: Some(true),
        };
        let commands = [
            DialogCommand::OpenFile(dialog_request.clone()),
            DialogCommand::OpenFiles(dialog_request.clone()),
            DialogCommand::ChooseFolder(dialog_request.clone()),
            DialogCommand::ChooseFolders(dialog_request.clone()),
            DialogCommand::SaveFile(dialog_request),
        ];

        for (index, command) in commands.into_iter().enumerate() {
            let request = CefariIpcRequest {
                id: format!("dialog-{index}"),
                command: CefariIpcCommand::Dialog(command.clone()),
            };
            let json = serde_json::to_string(&request).expect("request should serialize");

            assert_eq!(
                serde_json::from_str::<CefariIpcRequest>(&json)
                    .expect("request should deserialize"),
                request
            );
            assert!(json.contains("dialog"));
        }
    }

    #[test]
    fn round_trips_download_commands() {
        let request = CefariIpcRequest {
            id: "download-cancel".to_owned(),
            command: CefariIpcCommand::Download(DownloadCommand::Cancel(DownloadIdRequest {
                id: "cef-1".to_owned(),
            })),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcRequest>(&json).expect("request should deserialize"),
            request
        );
        assert!(json.contains("download"));
        assert!(json.contains("cancel"));
    }

    #[test]
    fn round_trips_dialog_results() {
        let canceled = DialogResult::Canceled;
        let selected = DialogResult::Selected {
            paths: vec![DialogSelectedPath {
                path: "/tmp/report.png".to_owned(),
                name: "report.png".to_owned(),
                kind: FileKind::File,
            }],
        };

        for result in [canceled, selected] {
            let json = serde_json::to_string(&result).expect("result should serialize");

            assert_eq!(
                serde_json::from_str::<DialogResult>(&json).expect("result should deserialize"),
                result
            );
        }
    }

    #[test]
    fn exports_typescript_bindings() {
        let output = specta_typescript::Typescript::default()
            .export(&ipc_types(), specta_serde::Format)
            .expect("TypeScript export should succeed");

        assert!(output.contains("export type CefariIpcRequest"));
        assert!(output.contains("windowSetTitle"));
        assert!(output.contains("unknownCommand"));
        assert!(output.contains("download"));
        assert!(output.contains("openFiles"));
        assert!(output.contains("chooseFolders"));
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
        let event = CefariIpcEvent::Download(DownloadEvent::Completed(DownloadCompletedEvent {
            id: "cef-1".to_owned(),
            url: "https://example.test/file.txt".to_owned(),
            destination_path: "/tmp/file.txt".to_owned(),
            received_bytes: 10.0,
            total_bytes: Some(10.0),
        }));
        let json = serde_json::to_string(&event).expect("event should serialize");

        assert_eq!(
            serde_json::from_str::<CefariIpcEvent>(&json).expect("event should deserialize"),
            event
        );
    }

    #[test]
    fn serializes_deep_link_opened_events() {
        let event = CefariIpcEvent::DeepLinkOpened(DeepLinkOpenEvent {
            url: "myapp://open/item?id=1".to_owned(),
        });

        let json = serde_json::to_string(&event).expect("event should serialize");

        assert_eq!(
            json,
            r#"{"event":"deepLinkOpened","payload":{"url":"myapp://open/item?id=1"}}"#
        );
        assert_eq!(
            serde_json::from_str::<CefariIpcEvent>(&json).expect("event should deserialize"),
            event
        );
    }
}
