use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{CefariTarget, PlatformSupport};

pub mod app;
pub mod dialogs;
pub mod downloads;
pub mod files;
pub mod notifications;
pub mod service;
pub mod shell;
pub mod tray;
pub mod updates;
pub mod windows;
pub mod workers;

pub use dialogs::*;
pub use downloads::*;
pub use files::*;
pub use notifications::*;
pub use service::*;
pub use shell::*;
pub use tray::*;
pub use updates::*;
pub use windows::*;
pub use workers::*;

include!(concat!(env!("OUT_DIR"), "/ipc_generated.rs"));

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IpcCapabilitySupport {
    pub name: &'static str,
    pub support: PlatformSupport,
    pub targets: &'static [CefariTarget],
    pub rationale: &'static str,
}

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
#[serde(tag = "code", content = "details", rename_all = "camelCase")]
pub enum CefariIpcError {
    InvalidCommand { message: String },
    Denied { message: String },
    UnknownCommand { command: String },
    Unsupported { command: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::{
        CefariIpcCommand, CefariIpcError, CefariIpcEvent, CefariIpcOutcome, CefariIpcRequest,
        CefariIpcResponse, DeepLinkOpenEvent, DialogCommand, DialogDefaultDirectory, DialogFilter,
        DialogModality, DialogRequest, DialogResult, DialogSelectedPath, DownloadCommand,
        DownloadCompletedEvent, DownloadEvent, DownloadIdRequest, FileKind, NotificationCommand,
        OpenExternalUrlRequest, WindowIdEvent, WindowSetTitleRequest, ipc_capability_support,
        ipc_types,
    };
    use crate::{CefariTarget, PlatformSupport};

    #[test]
    fn serializes_command_payloads_with_stable_tags() {
        let request = CefariIpcRequest {
            id: "request-1".to_owned(),
            command: CefariIpcCommand::WindowSetTitle(WindowSetTitleRequest {
                target: None,
                title: "Dashboard".to_owned(),
            }),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");

        assert_eq!(
            json,
            r#"{"id":"request-1","command":{"command":"windowSetTitle","payload":{"target":null,"title":"Dashboard"}}}"#
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
        let checked_in = include_str!("../../bindings/ipc.ts");

        assert_eq!(output, checked_in);
    }

    #[test]
    fn declares_platform_support_for_all_capabilities() {
        let support = ipc_capability_support();
        let names = support
            .iter()
            .map(|capability| capability.name)
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(
            support.len(),
            names.len(),
            "capability names must be unique"
        );
        assert_eq!(
            names,
            std::collections::BTreeSet::from([
                "app",
                "dialogs",
                "downloads",
                "files",
                "notifications",
                "service",
                "shell",
                "tray",
                "updates",
                "windows",
                "workers",
            ])
        );
        for capability in support {
            assert!(
                !capability.targets.is_empty(),
                "{} should declare at least one supported target",
                capability.name
            );
            assert!(
                !capability.rationale.is_empty(),
                "{} should explain its support classification",
                capability.name
            );
        }
    }

    #[test]
    fn classifies_desktop_only_capabilities() {
        let support = ipc_capability_support();

        for name in ["service", "tray", "updates", "windows"] {
            let capability = support
                .iter()
                .find(|capability| capability.name == name)
                .expect("capability should exist");
            assert_eq!(capability.support, PlatformSupport::DesktopOnly);
            assert_eq!(capability.targets, &[CefariTarget::Desktop]);
        }
    }

    #[test]
    fn event_types_round_trip() {
        let events = [
            CefariIpcEvent::Download(DownloadEvent::Completed(DownloadCompletedEvent {
                id: "cef-1".to_owned(),
                url: "https://example.test/file.txt".to_owned(),
                destination_path: "/tmp/file.txt".to_owned(),
                received_bytes: 10.0,
                total_bytes: Some(10.0),
            })),
            CefariIpcEvent::WindowClosed(WindowIdEvent {
                window_id: "main".to_owned(),
            }),
        ];

        for event in events {
            let json = serde_json::to_string(&event).expect("event should serialize");

            assert_eq!(
                serde_json::from_str::<CefariIpcEvent>(&json).expect("event should deserialize"),
                event
            );
        }
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
