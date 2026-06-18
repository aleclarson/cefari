use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowTargetRequest {
    pub target: Option<WindowTarget>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowTarget {
    pub id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowCreateRequest {
    pub id: Option<String>,
    pub route: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub visible: Option<bool>,
    pub focused: Option<bool>,
    pub resizable: Option<bool>,
    pub decorations: Option<bool>,
    pub always_on_top: Option<bool>,
    pub parent_id: Option<String>,
    pub modal: Option<bool>,
    pub persist_key: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowSetTitleRequest {
    pub target: Option<WindowTarget>,
    pub title: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowListResult {
    pub windows: Vec<WindowState>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowStateEvent {
    pub window_id: String,
    pub state: WindowState,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowIdEvent {
    pub window_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub id: String,
    pub kind: WindowKind,
    pub visible: bool,
    pub focused: bool,
    pub title: String,
    pub modal: bool,
    pub parent_id: Option<String>,
    pub route: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    Main,
    Secondary,
}
