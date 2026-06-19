use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "worker", content = "payload", rename_all = "camelCase")]
pub enum WorkerCommand {
    Spawn(WorkerSpawnRequest),
    Invoke(WorkerInvokeRequest),
    Terminate(WorkerIdRequest),
    List,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSpawnRequest {
    pub worker: String,
    pub input_json: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerIdRequest {
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerInvokeRequest {
    pub id: String,
    pub method: String,
    pub input_json: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerIdResult {
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum WorkerResult {
    Spawned(WorkerSpawnResult),
    Invoked(WorkerInvokeResult),
    Terminated(WorkerIdResult),
    List(WorkerListResult),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSpawnResult {
    pub id: String,
    pub worker: String,
    pub status: WorkerStatus,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerInvokeResult {
    pub id: String,
    pub method: String,
    pub output_json: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerListResult {
    pub workers: Vec<WorkerState>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerState {
    pub id: String,
    pub worker: String,
    pub status: WorkerStatus,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WorkerStatus {
    Running,
    Exited,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "event", content = "payload", rename_all = "camelCase")]
pub enum WorkerEvent {
    Message(WorkerMessageEvent),
    Exited(WorkerExitEvent),
    Error(WorkerErrorEvent),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerMessageEvent {
    pub id: String,
    pub worker: String,
    pub request_id: Option<String>,
    pub method: Option<String>,
    pub message_json: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerExitEvent {
    pub id: String,
    pub worker: String,
    pub code: Option<i32>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkerErrorEvent {
    pub id: String,
    pub worker: String,
    pub request_id: Option<String>,
    pub method: Option<String>,
    pub message: String,
}
