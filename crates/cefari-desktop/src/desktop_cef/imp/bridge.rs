use std::sync::{Arc, Mutex};

use tracing::{error, warn};

use cef::wrapper::message_router::{BrowserSideCallback, BrowserSideHandler, MessageRouterConfig};
use cef::{ImplBrowser as _, ImplFrame as _};

use crate::desktop_bridge::{
    BridgeOriginPolicy, denied_response_json, origin_from_url, transport_error_response_json,
};

use super::navigation::cef_userfree_string;
use super::runtime::CefBridgeIpcRequest;
use super::state::SharedBridgeIpcState;

pub(super) fn bridge_router_config() -> MessageRouterConfig {
    MessageRouterConfig {
        js_query_function: "__CEFARI_IPC_QUERY__".to_owned(),
        js_cancel_function: "__CEFARI_IPC_QUERY_CANCEL__".to_owned(),
        ..Default::default()
    }
}

pub(super) struct CefariBridgeIpcHandler {
    bridge_ipc: SharedBridgeIpcState,
    origin_policy: BridgeOriginPolicy,
}

impl CefariBridgeIpcHandler {
    pub(super) fn new(bridge_ipc: SharedBridgeIpcState, origin_policy: BridgeOriginPolicy) -> Self {
        Self {
            bridge_ipc,
            origin_policy,
        }
    }
}

impl BrowserSideHandler for CefariBridgeIpcHandler {
    fn on_query_str(
        &self,
        browser: Option<cef::Browser>,
        frame: Option<cef::Frame>,
        _query_id: i64,
        request: &str,
        _persistent: bool,
        callback: Arc<Mutex<dyn BrowserSideCallback>>,
    ) -> bool {
        let frame_url = frame
            .as_ref()
            .map(|frame| cef_userfree_string(&frame.url()))
            .unwrap_or_default();
        let origin = origin_from_url(&frame_url).unwrap_or_default();
        if !self.origin_policy.is_trusted_origin(&origin) {
            let response =
                denied_response_json(request, "origin is not allowed to use the Cefari bridge");
            if let Ok(callback) = callback.lock() {
                callback.success_str(&response);
            }
            warn!(
                frame_url,
                origin, "denied CEF bridge IPC from untrusted frame"
            );
            return true;
        }

        if let Err(error) = self.bridge_ipc.send(CefBridgeIpcRequest {
            browser_identifier: browser.as_ref().map(cef::Browser::identifier),
            origin,
            request_json: request.to_owned(),
            callback: callback.clone(),
        }) {
            if let Ok(callback) = callback.lock() {
                let response =
                    transport_error_response_json(request, "native IPC transport failed");
                callback.success_str(&response);
            }
            error!(
                %error,
                frame_url,
                "failed to enqueue CEF bridge IPC request"
            );
        }

        true
    }
}
