use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

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
