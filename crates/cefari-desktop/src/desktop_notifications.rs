use std::{
    collections::{BTreeMap, HashMap},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use cefari_core::{
    ActiveNotification, AppConfig, NotificationAction, NotificationCapabilities,
    NotificationCategory, NotificationCategoryAction, NotificationMediaReference,
    NotificationResponseEvent, NotificationSendRequest, NotificationXdgCategory, RuntimePaths,
};
use user_notify::{
    NotificationBuilder, NotificationCategory as UserNotificationCategory,
    NotificationCategoryAction as UserNotificationCategoryAction, NotificationManager,
    NotificationResponse, NotificationResponseAction as UserNotificationResponseAction,
    XdgNotificationCategory, get_notification_manager,
};

#[derive(Debug)]
pub struct DesktopNotifier {
    app_id: String,
    app_name: String,
    paths: RuntimePaths,
    manager: Arc<dyn NotificationManager>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NotificationRequest {
    title: String,
    body: Option<String>,
    subtitle: Option<String>,
    image: Option<PathBuf>,
    icon: Option<PathBuf>,
    icon_round_crop: bool,
    thread_id: Option<String>,
    category_id: Option<String>,
    user_info: BTreeMap<String, String>,
    xdg_category: Option<NotificationXdgCategory>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NotificationSendOutcome {
    Delivered { id: String },
    PermissionDenied,
}

impl DesktopNotifier {
    pub fn from_app_config(config: &AppConfig, paths: &RuntimePaths) -> Result<Self> {
        let app_id = required_notification_field(&config.identifier, "app identifier")?;
        let app_name = required_notification_field(&config.display_name, "app display name")?;
        let manager = get_notification_manager(app_id.clone(), None);

        Ok(Self {
            app_id,
            app_name,
            paths: paths.clone(),
            manager,
        })
    }

    #[cfg(test)]
    fn with_manager_for_tests(
        config: &AppConfig,
        paths: RuntimePaths,
        manager: Arc<dyn NotificationManager>,
    ) -> Result<Self> {
        let app_id = required_notification_field(&config.identifier, "app identifier")?;
        let app_name = required_notification_field(&config.display_name, "app display name")?;

        Ok(Self {
            app_id,
            app_name,
            paths,
            manager,
        })
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    #[allow(dead_code)]
    pub fn capabilities(&self) -> NotificationCapabilities {
        platform_capabilities()
    }

    #[allow(dead_code)]
    pub fn register_categories(&self, categories: &[NotificationCategory]) -> Result<u32> {
        let converted = categories
            .iter()
            .map(notification_category_to_user_notify)
            .collect::<Result<Vec<_>>>()?;
        self.manager
            .register(
                Box::new(|response| tracing::debug!(?response, "notification response")),
                converted,
            )
            .context("failed to register desktop notification categories")?;
        categories
            .len()
            .try_into()
            .context("notification category count exceeds IPC range")
    }

    #[allow(dead_code)]
    pub async fn permission_allowed(&self) -> Result<bool> {
        self.manager
            .get_notification_permission_state()
            .await
            .context("failed to read desktop notification permission state")
    }

    #[allow(dead_code)]
    pub async fn request_permission_once(&self) -> Result<bool> {
        self.manager
            .first_time_ask_for_notification_permission()
            .await
            .context("failed to request desktop notification permission")
    }

    #[allow(dead_code)]
    pub async fn send(&self, request: &NotificationSendRequest) -> Result<NotificationSendOutcome> {
        if !self.permission_allowed().await? {
            return Ok(NotificationSendOutcome::PermissionDenied);
        }

        let request = NotificationRequest::from_ipc(request, &self.paths)?;
        let handle = self
            .manager
            .send_notification(request.to_user_notify_builder(&self.app_name))
            .await
            .context("failed to send desktop notification")?;

        Ok(NotificationSendOutcome::Delivered {
            id: handle.get_id(),
        })
    }

    #[allow(dead_code)]
    pub async fn active_notifications(&self) -> Result<Vec<ActiveNotification>> {
        self.manager
            .get_active_notifications()
            .await
            .context("failed to read active desktop notifications")
            .map(|notifications| {
                notifications
                    .into_iter()
                    .map(|notification| ActiveNotification {
                        id: notification.get_id(),
                        user_info: notification
                            .get_user_info()
                            .iter()
                            .map(|(key, value)| (key.clone(), value.clone()))
                            .collect(),
                    })
                    .collect()
            })
    }

    #[allow(dead_code)]
    pub fn remove_delivered(&self, ids: &[String]) -> Result<u32> {
        let borrowed = ids.iter().map(String::as_str).collect::<Vec<_>>();
        self.manager
            .remove_delivered_notifications(borrowed)
            .context("failed to remove delivered desktop notifications")?;
        ids.len()
            .try_into()
            .context("notification removal count exceeds IPC range")
    }

    #[allow(dead_code)]
    pub async fn remove_all_delivered(&self) -> Result<u32> {
        let count = self
            .active_notifications()
            .await?
            .len()
            .try_into()
            .context("notification removal count exceeds IPC range")?;
        self.manager
            .remove_all_delivered_notifications()
            .context("failed to remove all delivered desktop notifications")?;
        Ok(count)
    }
}

impl NotificationRequest {
    #[allow(dead_code)]
    pub fn new(title: impl Into<String>) -> Result<Self> {
        let title = required_notification_field(&title.into(), "notification title")?;

        Ok(Self {
            title,
            body: None,
            subtitle: None,
            image: None,
            icon: None,
            icon_round_crop: false,
            thread_id: None,
            category_id: None,
            user_info: BTreeMap::new(),
            xdg_category: None,
        })
    }

    pub fn from_ipc(request: &NotificationSendRequest, paths: &RuntimePaths) -> Result<Self> {
        Ok(Self {
            title: required_notification_field(&request.title, "notification title")?,
            body: optional_notification_field(request.body.as_deref(), "notification body")?,
            subtitle: optional_notification_field(
                request.subtitle.as_deref(),
                "notification subtitle",
            )?,
            image: optional_media_path(request.image.as_ref(), paths)?,
            icon: optional_media_path(request.icon.as_ref(), paths)?,
            icon_round_crop: request.icon_round_crop,
            thread_id: optional_notification_field(
                request.thread_id.as_deref(),
                "notification thread id",
            )?,
            category_id: optional_notification_field(
                request.category_id.as_deref(),
                "notification category id",
            )?,
            user_info: request.user_info.clone(),
            xdg_category: request.xdg_category.clone(),
        })
    }

    #[allow(dead_code)]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[allow(dead_code)]
    pub fn body(mut self, body: impl Into<String>) -> Result<Self> {
        self.body = Some(required_notification_field(
            &body.into(),
            "notification body",
        )?);
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Result<Self> {
        self.subtitle = Some(required_notification_field(
            &subtitle.into(),
            "notification subtitle",
        )?);
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn thread_id(mut self, thread_id: impl Into<String>) -> Result<Self> {
        self.thread_id = Some(required_notification_field(
            &thread_id.into(),
            "notification thread id",
        )?);
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn category_id(mut self, category_id: impl Into<String>) -> Result<Self> {
        self.category_id = Some(required_notification_field(
            &category_id.into(),
            "notification category id",
        )?);
        Ok(self)
    }

    fn to_user_notify_builder(&self, app_name: &str) -> NotificationBuilder {
        let mut builder = NotificationBuilder::new()
            .title(&self.title)
            .set_xdg_app_name(app_name.to_owned())
            .set_icon_round_crop(self.icon_round_crop);

        if let Some(body) = &self.body {
            builder = builder.body(body);
        }

        if let Some(subtitle) = &self.subtitle {
            builder = builder.subtitle(subtitle);
        }

        if let Some(image) = &self.image {
            builder = builder.set_image(image.clone());
        }

        if let Some(icon) = &self.icon {
            builder = builder.set_icon(icon.clone());
        }

        if let Some(thread_id) = &self.thread_id {
            builder = builder.set_thread_id(thread_id);
        }

        if let Some(category_id) = &self.category_id {
            builder = builder.set_category_id(category_id);
        }

        if !self.user_info.is_empty() {
            builder = builder.set_user_info(
                self.user_info
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<HashMap<_, _>>(),
            );
        }

        if let Some(category) = &self.xdg_category {
            builder = builder.set_xdg_category(xdg_category_to_user_notify(category));
        }

        builder
    }
}

fn notification_category_to_user_notify(
    category: &NotificationCategory,
) -> Result<UserNotificationCategory> {
    Ok(UserNotificationCategory {
        identifier: required_notification_field(&category.id, "notification category id")?,
        actions: category
            .actions
            .iter()
            .map(notification_action_to_user_notify)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn notification_action_to_user_notify(
    action: &NotificationCategoryAction,
) -> Result<UserNotificationCategoryAction> {
    match action {
        NotificationCategoryAction::Action { id, title } => {
            Ok(UserNotificationCategoryAction::Action {
                identifier: required_notification_field(id, "notification action id")?,
                title: required_notification_field(title, "notification action title")?,
            })
        }
        NotificationCategoryAction::TextInput {
            id,
            title,
            input_button_title,
            input_placeholder,
        } => Ok(UserNotificationCategoryAction::TextInputAction {
            identifier: required_notification_field(id, "notification text input action id")?,
            title: required_notification_field(title, "notification text input action title")?,
            input_button_title: required_notification_field(
                input_button_title,
                "notification text input button title",
            )?,
            input_placeholder: required_notification_field(
                input_placeholder,
                "notification text input placeholder",
            )?,
        }),
    }
}

#[allow(dead_code)]
pub fn notification_response_event(response: &NotificationResponse) -> NotificationResponseEvent {
    notification_response_event_parts(
        response.notification_id.clone(),
        &response.action,
        response.user_text.clone(),
        response
            .user_info
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

fn notification_response_event_parts(
    id: String,
    action: &UserNotificationResponseAction,
    user_text: Option<String>,
    user_info: BTreeMap<String, String>,
) -> NotificationResponseEvent {
    NotificationResponseEvent {
        id,
        action: notification_action_from_user_notify(action),
        user_text,
        user_info,
    }
}

fn notification_action_from_user_notify(
    action: &UserNotificationResponseAction,
) -> NotificationAction {
    match action {
        UserNotificationResponseAction::Default => NotificationAction::Default,
        UserNotificationResponseAction::Dismiss => NotificationAction::Dismiss,
        UserNotificationResponseAction::Other(action) => NotificationAction::Other(action.clone()),
    }
}

fn optional_media_path(
    reference: Option<&NotificationMediaReference>,
    paths: &RuntimePaths,
) -> Result<Option<PathBuf>> {
    reference
        .map(|reference| resolve_media_path(reference, paths))
        .transpose()
}

fn resolve_media_path(
    reference: &NotificationMediaReference,
    paths: &RuntimePaths,
) -> Result<PathBuf> {
    match reference {
        NotificationMediaReference::AppResource(path) => safe_child_path(
            &paths.resource_dir.join("frontend"),
            path,
            "notification app resource",
        ),
        NotificationMediaReference::AppData(path) => {
            safe_child_path(&paths.data_dir, path, "notification app data")
        }
    }
}

fn safe_child_path(root: &Path, path: &str, label: &str) -> Result<PathBuf> {
    let path = required_notification_field(path, label)?;
    let relative = Path::new(&path);
    if relative.is_absolute() {
        bail!("{label} path must be relative");
    }

    let mut resolved = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("{label} path must stay inside its Cefari root");
            }
        }
    }

    Ok(resolved)
}

fn optional_notification_field(value: Option<&str>, field: &str) -> Result<Option<String>> {
    value
        .map(|value| required_notification_field(value, field))
        .transpose()
}

fn required_notification_field(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(value.to_owned())
}

fn xdg_category_to_user_notify(category: &NotificationXdgCategory) -> XdgNotificationCategory {
    match category {
        NotificationXdgCategory::Call => XdgNotificationCategory::Call,
        NotificationXdgCategory::CallEnded => XdgNotificationCategory::CallEnded,
        NotificationXdgCategory::CallIncoming => XdgNotificationCategory::CallIncoming,
        NotificationXdgCategory::CallUnanswered => XdgNotificationCategory::CallUnanswered,
        NotificationXdgCategory::Device => XdgNotificationCategory::Device,
        NotificationXdgCategory::DeviceAdded => XdgNotificationCategory::DeviceAdded,
        NotificationXdgCategory::DeviceError => XdgNotificationCategory::DeviceError,
        NotificationXdgCategory::DeviceRemoved => XdgNotificationCategory::DeviceRemoved,
        NotificationXdgCategory::Email => XdgNotificationCategory::Email,
        NotificationXdgCategory::EmailArrived => XdgNotificationCategory::EmailArrived,
        NotificationXdgCategory::EmailBounced => XdgNotificationCategory::EmailBounced,
        NotificationXdgCategory::Im => XdgNotificationCategory::Im,
        NotificationXdgCategory::ImError => XdgNotificationCategory::ImError,
        NotificationXdgCategory::ImReceived => XdgNotificationCategory::ImReceived,
        NotificationXdgCategory::Network => XdgNotificationCategory::Network,
        NotificationXdgCategory::NetworkConnected => XdgNotificationCategory::NetworkConnected,
        NotificationXdgCategory::NetworkDisconnected => {
            XdgNotificationCategory::NetworkDisconnected
        }
        NotificationXdgCategory::NetworkError => XdgNotificationCategory::NetworkError,
        NotificationXdgCategory::Presence => XdgNotificationCategory::Presence,
        NotificationXdgCategory::PresenceOffline => XdgNotificationCategory::PresenceOffline,
        NotificationXdgCategory::PresenceOnline => XdgNotificationCategory::PresenceOnline,
        NotificationXdgCategory::Transfer => XdgNotificationCategory::Transfer,
        NotificationXdgCategory::TransferComplete => XdgNotificationCategory::TransferComplete,
        NotificationXdgCategory::TransferError => XdgNotificationCategory::TransferError,
    }
}

fn platform_capabilities() -> NotificationCapabilities {
    NotificationCapabilities {
        permission_state: cfg!(target_os = "macos"),
        permission_prompt: cfg!(target_os = "macos"),
        subtitle: cfg!(any(target_os = "macos", target_os = "windows")),
        image: true,
        icon: !cfg!(target_os = "macos"),
        icon_round_crop: cfg!(target_os = "windows"),
        thread_id: cfg!(target_os = "macos"),
        categories: true,
        action_buttons: cfg!(target_os = "macos"),
        text_input_actions: cfg!(target_os = "macos"),
        user_info: true,
        xdg_category: cfg!(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )),
        active_notifications: true,
        remove_delivered: true,
        response_events: true,
        cold_start_activation: cfg!(any(target_os = "macos", target_os = "windows")),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

    use cefari_core::{
        AppConfig, NotificationCategory, NotificationCategoryAction, NotificationMediaReference,
        NotificationSendRequest, NotificationXdgCategory, RuntimePaths,
    };
    use user_notify::{NotificationResponseAction, mock::NotificationManagerMock};

    use super::{
        DesktopNotifier, NotificationRequest, NotificationSendOutcome,
        notification_response_event_parts,
    };

    #[test]
    fn notification_request_requires_a_title() {
        let error = NotificationRequest::new("  ").expect_err("blank titles should be rejected");

        assert!(
            error
                .to_string()
                .contains("notification title cannot be empty")
        );
    }

    #[test]
    fn notification_request_trims_user_visible_fields() {
        let request = NotificationRequest::new("  Build finished  ")
            .expect("title should be valid")
            .body("  App package is ready  ")
            .expect("body should be valid");

        assert_eq!(request.title(), "Build finished");
        assert_eq!(request.body.as_deref(), Some("App package is ready"));
    }

    #[test]
    fn notification_request_rejects_blank_optional_text() {
        let request = notification_send_request();
        let mut blank_body = request.clone();
        blank_body.body = Some("  ".to_owned());

        let error = NotificationRequest::from_ipc(&blank_body, &test_paths())
            .expect_err("blank body should be rejected");

        assert!(
            error
                .to_string()
                .contains("notification body cannot be empty")
        );
    }

    #[test]
    fn notification_request_resolves_cefari_media_references() {
        let paths = test_paths();
        let request = NotificationRequest::from_ipc(&notification_send_request(), &paths)
            .expect("request should be valid");

        assert_eq!(
            request.image,
            Some(paths.resource_dir.join("frontend/images/build.png"))
        );
        assert_eq!(request.icon, Some(paths.data_dir.join("icons/build.png")));
        assert_eq!(request.user_info.get("buildId"), Some(&"123".to_owned()));
        assert_eq!(
            request.xdg_category,
            Some(NotificationXdgCategory::TransferComplete)
        );
    }

    #[test]
    fn notification_request_rejects_unsafe_media_paths() {
        let mut request = notification_send_request();
        request.image = Some(NotificationMediaReference::AppResource(
            "../secrets.png".to_owned(),
        ));

        let error = NotificationRequest::from_ipc(&request, &test_paths())
            .expect_err("parent traversal should be rejected");

        assert!(
            error
                .to_string()
                .contains("notification app resource path must stay inside")
        );
    }

    #[test]
    fn notification_category_requires_valid_action_fields() {
        let notifier = test_notifier();
        let categories = [NotificationCategory {
            id: "message".to_owned(),
            actions: vec![NotificationCategoryAction::TextInput {
                id: "reply".to_owned(),
                title: "Reply".to_owned(),
                input_button_title: "  ".to_owned(),
                input_placeholder: "Message".to_owned(),
            }],
        }];

        let error = notifier
            .register_categories(&categories)
            .expect_err("blank action labels should be rejected");

        assert!(
            error
                .to_string()
                .contains("notification text input button title cannot be empty")
        );
    }

    #[test]
    fn maps_notification_responses_to_ipc_events() {
        let event = notification_response_event_parts(
            "n1".to_owned(),
            &NotificationResponseAction::Other("reply".to_owned()),
            Some("Hello".to_owned()),
            BTreeMap::from([("messageId".to_owned(), "m1".to_owned())]),
        );

        assert_eq!(event.id, "n1");
        assert_eq!(
            event.action,
            cefari_core::NotificationAction::Other("reply".to_owned())
        );
        assert_eq!(event.user_text.as_deref(), Some("Hello"));
        assert_eq!(event.user_info.get("messageId"), Some(&"m1".to_owned()));
    }

    #[test]
    fn desktop_notifier_requires_app_identity() {
        let config = AppConfig {
            identifier: "  ".to_owned(),
            display_name: "Cefari".to_owned(),
            version: "1.2.3".to_owned(),
        };
        let error = DesktopNotifier::from_app_config(&config, &test_paths())
            .expect_err("blank app id should fail");

        assert!(error.to_string().contains("app identifier cannot be empty"));
    }

    #[test]
    fn mock_manager_exercises_delivery_and_management() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio runtime should build");
        let notifier = test_notifier();

        runtime.block_on(async {
            let count = notifier
                .register_categories(&[NotificationCategory {
                    id: "message".to_owned(),
                    actions: vec![NotificationCategoryAction::Action {
                        id: "open".to_owned(),
                        title: "Open".to_owned(),
                    }],
                }])
                .expect("categories should register");
            assert_eq!(count, 1);

            let outcome = notifier
                .send(&notification_send_request())
                .await
                .expect("notification should send");
            let NotificationSendOutcome::Delivered { id } = outcome else {
                panic!("mock manager should allow notifications");
            };

            let active = notifier
                .active_notifications()
                .await
                .expect("active notifications should load");
            assert_eq!(active.len(), 1);
            assert_eq!(active[0].user_info.get("buildId"), Some(&"123".to_owned()));

            assert_eq!(
                notifier
                    .remove_delivered(std::slice::from_ref(&id))
                    .expect("notification should be removed"),
                1
            );
            assert!(
                notifier
                    .active_notifications()
                    .await
                    .expect("active notifications should reload")
                    .is_empty()
            );

            notifier
                .send(&notification_send_request())
                .await
                .expect("first notification should send");
            notifier
                .send(&notification_send_request())
                .await
                .expect("second notification should send");
            assert_eq!(
                notifier
                    .remove_all_delivered()
                    .await
                    .expect("all notifications should be removed"),
                2
            );
        });
    }

    fn test_notifier() -> DesktopNotifier {
        DesktopNotifier::with_manager_for_tests(
            &AppConfig {
                identifier: "dev.cefari.test".to_owned(),
                display_name: "Cefari Test".to_owned(),
                version: "1.2.3".to_owned(),
            },
            test_paths(),
            Arc::new(NotificationManagerMock::new()),
        )
        .expect("test notifier should build")
    }

    fn notification_send_request() -> NotificationSendRequest {
        NotificationSendRequest {
            title: "  Build finished  ".to_owned(),
            body: Some("  App package is ready  ".to_owned()),
            subtitle: Some("  Release  ".to_owned()),
            image: Some(NotificationMediaReference::AppResource(
                "images/build.png".to_owned(),
            )),
            icon: Some(NotificationMediaReference::AppData(
                "icons/build.png".to_owned(),
            )),
            icon_round_crop: true,
            thread_id: Some("  builds  ".to_owned()),
            category_id: Some("  message  ".to_owned()),
            user_info: [("buildId".to_owned(), "123".to_owned())].into(),
            xdg_category: Some(NotificationXdgCategory::TransferComplete),
        }
    }

    fn test_paths() -> RuntimePaths {
        let root = PathBuf::from("/tmp/cefari-notification-test");
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config/cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("data/logs"),
            resource_dir: root.join("resources"),
            update_dir: root.join("data/updates"),
        }
    }
}
