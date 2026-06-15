use tracing::{debug, info, warn};

use cef::{ImplFrame as _, ImplProcessMessage as _, ImplRequest as _};

use crate::desktop_bridge::{
    BridgeOriginPolicy, NavigationDecision, NavigationPolicy, NavigationSurface, origin_from_url,
};
use crate::external;

pub(super) fn optional_cef_string(value: Option<&cef::CefString>) -> String {
    value.map(ToString::to_string).unwrap_or_default()
}

pub(super) fn cef_userfree_string(value: &cef::CefStringUserfree) -> String {
    let value: Option<&cef::sys::_cef_string_utf16_t> = value.into();
    let Some(value) = value else {
        return String::new();
    };
    if value.str_.is_null() || value.length == 0 {
        return String::new();
    }

    cef::CefString::from(*value).to_string()
}

pub(super) fn frame_url(frame: Option<&mut cef::Frame>) -> String {
    frame
        .map(|frame| cef_userfree_string(&frame.url()))
        .unwrap_or_default()
}

pub(super) fn request_url(request: Option<&mut cef::Request>) -> String {
    request
        .map(|request| cef_userfree_string(&request.url()))
        .unwrap_or_default()
}

pub(super) fn message_name(message: Option<&mut cef::ProcessMessage>) -> String {
    message
        .map(|message| cef_userfree_string(&message.name()))
        .unwrap_or_default()
}

pub(super) fn handle_navigation_decision(
    policy: &NavigationPolicy,
    surface: NavigationSurface,
    url: &str,
    operation: &str,
    details: &[(&str, String)],
) -> ::std::os::raw::c_int {
    let decision = policy.decide(surface, url);
    match decision.decision {
        NavigationDecision::Allow => {
            debug!(url, surface = ?surface, reason = decision.reason, operation, "allowed CEF navigation");
            0
        }
        NavigationDecision::OpenExternally => {
            match external::open_external_url(url) {
                Ok(()) => {
                    info!(
                        url,
                        surface = ?surface,
                        reason = decision.reason,
                        operation,
                        details = ?details,
                        "opened CEF navigation externally"
                    );
                }
                Err(error) => {
                    warn!(
                        url,
                        surface = ?surface,
                        reason = decision.reason,
                        operation,
                        details = ?details,
                        %error,
                        "failed to open CEF navigation externally"
                    );
                }
            }
            1
        }
        NavigationDecision::Deny => {
            warn!(
                url,
                surface = ?surface,
                reason = decision.reason,
                operation,
                details = ?details,
                "denied CEF navigation"
            );
            1
        }
    }
}

pub(super) fn inject_bridge_script(origin_policy: &BridgeOriginPolicy, frame: &mut cef::Frame) {
    if frame.is_main() == 0 {
        debug!("skipping Cefari bridge injection for non-main frame");
        return;
    }

    let frame_url = cef_userfree_string(&frame.url());
    let origin = origin_from_url(&frame_url);
    let Some(script) = origin_policy.bridge_script_for_url(&frame_url) else {
        debug!(
            frame_url,
            origin = origin.as_deref().unwrap_or_default(),
            "skipping Cefari bridge injection for untrusted frame"
        );
        return;
    };

    let code = cef::CefString::from(script);
    let script_url = cef::CefString::from("cefari://bridge/bootstrap.js");
    frame.execute_java_script(Some(&code), Some(&script_url), 1);
    info!(
        frame_url,
        origin = origin.as_deref().unwrap_or_default(),
        "Cefari bridge script injected"
    );
}
