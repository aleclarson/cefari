use serde::{Deserialize, Serialize};
use specta::Type;

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
