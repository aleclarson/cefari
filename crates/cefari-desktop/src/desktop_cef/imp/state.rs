use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

use cef::ImplBrowser as _;
use cef::wrapper::stream_resource_handler::StreamResourceHandler;

use crate::desktop_ui::diagnose_app_scheme_resource;

use super::{BridgeIpcSender, CefBridgeIpcRequest, MessagePumpScheduler};

#[derive(Clone, Default)]
pub(super) struct SharedBrowserState(Rc<RefCell<BrowserState>>);

#[allow(dead_code)]
impl SharedBrowserState {
    pub(super) fn has_browser(&self) -> bool {
        self.0.borrow().main_browser.is_some()
    }

    pub(super) fn active_browser(&self) -> Result<cef::Browser> {
        self.0
            .borrow()
            .main_browser
            .clone()
            .context("CEF main browser is not available")
    }

    pub(super) fn browser_created(&self, browser: &cef::Browser) {
        let identifier = browser.identifier();
        let is_popup = browser.is_popup() != 0;
        let mut state = self.0.borrow_mut();

        if state.main_browser.is_none() && !is_popup {
            state.main_browser = Some(browser.clone());
            info!(identifier, "CEF main browser retained");
        } else {
            debug!(
                identifier,
                is_popup,
                has_main_browser = state.main_browser.is_some(),
                "CEF browser created outside main-browser retention"
            );
        }
    }

    pub(super) fn browser_closing(&self, browser: &cef::Browser) {
        let identifier = browser.identifier();
        let mut state = self.0.borrow_mut();
        let should_clear = state
            .main_browser
            .as_ref()
            .is_some_and(|main_browser| main_browser.identifier() == identifier);

        if should_clear {
            state.main_browser = None;
            info!(identifier, "CEF main browser released");
        } else {
            debug!(identifier, "CEF non-main browser closing");
        }
    }
}

#[derive(Default)]
struct BrowserState {
    main_browser: Option<cef::Browser>,
}

#[derive(Clone, Default)]
pub(super) struct SharedBridgeIpcState(Arc<Mutex<BridgeIpcState>>);

impl SharedBridgeIpcState {
    pub(super) fn set_sender(&self, sender: Arc<dyn BridgeIpcSender>) {
        if let Ok(mut state) = self.0.lock() {
            state.sender = Some(sender);
        }
    }

    pub(super) fn send(&self, request: CefBridgeIpcRequest) -> Result<()> {
        let sender = self
            .0
            .lock()
            .ok()
            .and_then(|state| state.sender.clone())
            .context("CEF bridge IPC sender is not installed")?;
        sender.send_bridge_ipc(request)
    }
}

#[derive(Default)]
struct BridgeIpcState {
    sender: Option<Arc<dyn BridgeIpcSender>>,
}

#[derive(Clone, Default)]
pub(super) struct SharedMessagePumpState(Arc<Mutex<MessagePumpState>>);

impl SharedMessagePumpState {
    pub(super) fn set_scheduler(&self, scheduler: Arc<dyn MessagePumpScheduler>) {
        if let Ok(mut state) = self.0.lock() {
            state.scheduler = Some(scheduler);
        }
    }

    pub(super) fn schedule(&self, delay_ms: i64) {
        let scheduler = self.0.lock().ok().and_then(|state| state.scheduler.clone());
        let Some(scheduler) = scheduler else {
            debug!(
                delay_ms,
                "CEF message pump work scheduled before Tao scheduler was installed"
            );
            return;
        };

        if let Err(error) = scheduler.schedule_message_pump_work(delay_ms) {
            warn!(%error, delay_ms, "failed to schedule CEF message pump work");
        }
    }
}

#[derive(Default)]
struct MessagePumpState {
    scheduler: Option<Arc<dyn MessagePumpScheduler>>,
}

#[derive(Clone, Default)]
pub(super) struct SharedAppSchemeState(Arc<Mutex<AppSchemeState>>);

impl SharedAppSchemeState {
    pub(super) fn set_resource_dir(&self, resource_dir: PathBuf) {
        if let Ok(mut state) = self.0.lock() {
            state.resource_dir = Some(resource_dir);
        }
    }

    pub(super) fn resource_handler_for_url(
        &self,
        url: &str,
    ) -> Result<cef::ResourceHandler, String> {
        let resource_dir = self
            .0
            .lock()
            .ok()
            .and_then(|state| state.resource_dir.clone())
            .ok_or_else(|| "app-scheme resource root is not installed".to_owned())?;
        let resource =
            diagnose_app_scheme_resource(&resource_dir, url).map_err(|error| error.to_string())?;
        let path = resource.path.to_string_lossy();
        let stream = cef::stream_reader_create_for_file(Some(&cef::CefString::from(path.as_ref())))
            .ok_or_else(|| {
                format!(
                    "failed to open resource stream for {}",
                    resource.path.display()
                )
            })?;
        Ok(StreamResourceHandler::new_with_stream(
            resource.mime_type.to_owned(),
            stream,
        ))
    }
}

#[derive(Default)]
struct AppSchemeState {
    resource_dir: Option<PathBuf>,
}
