use std::sync::Arc;

use anyhow::{Context, Result, bail};
use cefari_core::AppConfig;
use user_notify::{NotificationBuilder, NotificationManager, get_notification_manager};

#[derive(Debug)]
pub struct DesktopNotifier {
    app_id: String,
    app_name: String,
    manager: Arc<dyn NotificationManager>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NotificationRequest {
    title: String,
    body: Option<String>,
    subtitle: Option<String>,
    thread_id: Option<String>,
    category_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NotificationSendOutcome {
    Delivered { id: String },
    PermissionDenied,
}

impl DesktopNotifier {
    pub fn from_app_config(config: &AppConfig) -> Result<Self> {
        let app_id = required_notification_field(&config.identifier, "app identifier")?;
        let app_name = required_notification_field(&config.display_name, "app display name")?;
        let manager = get_notification_manager(app_id.clone(), None);
        manager
            .register(
                Box::new(|response| tracing::debug!(?response, "notification response")),
                Vec::new(),
            )
            .context("failed to register desktop notification handler")?;

        Ok(Self {
            app_id,
            app_name,
            manager,
        })
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
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
    pub async fn send(&self, request: &NotificationRequest) -> Result<NotificationSendOutcome> {
        if !self.permission_allowed().await? {
            return Ok(NotificationSendOutcome::PermissionDenied);
        }

        let handle = self
            .manager
            .send_notification(request.to_user_notify_builder(&self.app_name))
            .await
            .context("failed to send desktop notification")?;

        Ok(NotificationSendOutcome::Delivered {
            id: handle.get_id(),
        })
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
            thread_id: None,
            category_id: None,
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
            .set_xdg_app_name(app_name.to_owned());

        if let Some(body) = &self.body {
            builder = builder.body(body);
        }

        if let Some(subtitle) = &self.subtitle {
            builder = builder.subtitle(subtitle);
        }

        if let Some(thread_id) = &self.thread_id {
            builder = builder.set_thread_id(thread_id);
        }

        if let Some(category_id) = &self.category_id {
            builder = builder.set_category_id(category_id);
        }

        builder
    }
}

fn required_notification_field(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use cefari_core::AppConfig;

    use super::{DesktopNotifier, NotificationRequest};

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
    fn desktop_notifier_requires_app_identity() {
        let config = AppConfig {
            identifier: "  ".to_owned(),
            display_name: "Cefari".to_owned(),
        };
        let error =
            DesktopNotifier::from_app_config(&config).expect_err("blank app id should fail");

        assert!(error.to_string().contains("app identifier cannot be empty"));
    }
}
