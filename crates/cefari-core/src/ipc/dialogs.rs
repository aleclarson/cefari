use serde::{Deserialize, Serialize};
use specta::Type;

use super::FileKind;

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
