#![allow(dead_code)]

use cefari_core::{CefariIpcError, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse};
use serde::Deserialize;
use serde_json::Value;

use crate::desktop_ipc::{DesktopIpcContext, DesktopIpcDispatcher};

pub const CEFARI_DEFAULT_DEV_PORT: u16 = 5173;
pub const CEFARI_DEFAULT_STYLES: &str = r".cefari-drag {
  -webkit-app-region: drag;
}

.cefari-no-drag,
.cefari-drag button,
.cefari-drag input,
.cefari-drag textarea,
.cefari-drag select,
.cefari-drag a {
  -webkit-app-region: no-drag;
}
";

pub const CEFARI_BRIDGE_SCRIPT: &str = r#"
(() => {
  if (window.cefari) return;

  const defaultStyles = `.cefari-drag {
  -webkit-app-region: drag;
}

.cefari-no-drag,
.cefari-drag button,
.cefari-drag input,
.cefari-drag textarea,
.cefari-drag select,
.cefari-drag a {
  -webkit-app-region: no-drag;
}
`;

  const installDefaultStyles = () => {
    if (document.getElementById("cefari-default-styles")) return;

    const style = document.createElement("style");
    style.id = "cefari-default-styles";
    style.dataset.cefariDefaultStyles = "true";
    style.textContent = defaultStyles;

    const target = document.head || document.documentElement;
    target.prepend(style);
  };

  installDefaultStyles();

  let nextId = 1;
  const listeners = new Set();
  const pendingEvents = [];
  const daemonStreamListeners = new Set();
  const pendingDaemonStreamEvents = [];

  const unsupported = (id, command, reason) => ({
    id,
    outcome: {
      status: "err",
      payload: {
        code: "unsupported",
        details: { command, reason },
      },
    },
  });

  const postNativeIpc = (request) => new Promise((resolve) => {
    const query = window.__CEFARI_IPC_QUERY__;
    if (typeof query !== "function") {
      resolve(unsupported(request.id, "bridge", "native IPC transport is unavailable"));
      return;
    }

    query({
      request: JSON.stringify(request),
      onSuccess(response) {
        try {
          resolve(JSON.parse(response));
        } catch (_error) {
          resolve(unsupported(request.id, "bridge", "native IPC response was invalid"));
        }
      },
      onFailure(_code, message) {
        resolve(unsupported(request.id, "bridge", message || "native IPC transport failed"));
      },
    });
  });

  Object.defineProperty(window, "cefari", {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({
      invoke(command) {
        const id = `cefari-${nextId++}`;
        const post = window.__CEFARI_IPC_POST__;
        if (typeof post !== "function") {
          return Promise.resolve(unsupported(id, "bridge", "native IPC transport is unavailable"));
        }
        return Promise.resolve(post({ id, command }));
      },
      on(handler) {
        listeners.add(handler);
        for (const event of pendingEvents.splice(0)) handler(event);
        return () => listeners.delete(handler);
      },
    }),
  });

  Object.defineProperty(window, "__CEFARI_IPC_EVENT__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value(event) {
      if (listeners.size === 0) {
        pendingEvents.push(event);
        return;
      }
      for (const listener of listeners) listener(event);
    },
  });

  Object.defineProperty(window, "__CEFARI_IPC_POST__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value: postNativeIpc,
  });

  Object.defineProperty(window, "__CEFARI_DAEMON_STREAM_POST__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value(command) {
      const id = `cefari-daemon-${nextId++}`;
      return postNativeIpc({ id, daemon: command });
    },
  });

  Object.defineProperty(window, "__CEFARI_DAEMON_STREAM_ON__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value(handler) {
      daemonStreamListeners.add(handler);
      for (const event of pendingDaemonStreamEvents.splice(0)) handler(event);
      return () => daemonStreamListeners.delete(handler);
    },
  });

  Object.defineProperty(window, "__CEFARI_DAEMON_STREAM_EVENT__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value(event) {
      if (daemonStreamListeners.size === 0) {
        pendingDaemonStreamEvents.push(event);
        return;
      }
      for (const listener of daemonStreamListeners) listener(event);
    },
  });
})();
"#;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BridgeOriginPolicy {
    trusted_packaged_origins: Vec<String>,
    allowed_dev_origins: Vec<String>,
}

impl BridgeOriginPolicy {
    pub fn from_environment() -> Self {
        let dev_port = std::env::var("CEFARI_FRONTEND_PORT")
            .ok()
            .and_then(|port| port.parse::<u16>().ok())
            .or_else(|| {
                std::env::var("CEFARI_FRONTEND_URL")
                    .ok()
                    .and_then(|url| dev_port_from_url(&url))
            })
            .unwrap_or(CEFARI_DEFAULT_DEV_PORT);

        Self::for_dev_port(dev_port)
    }

    pub fn for_dev_port(dev_port: u16) -> Self {
        Self {
            trusted_packaged_origins: vec!["cefari://app".to_owned(), "app://cefari".to_owned()],
            allowed_dev_origins: vec![
                format!("http://127.0.0.1:{dev_port}"),
                format!("http://localhost:{dev_port}"),
            ],
        }
    }

    pub fn is_trusted_origin(&self, origin: &str) -> bool {
        self.trusted_packaged_origins
            .iter()
            .any(|trusted| origin == trusted)
            || self
                .allowed_dev_origins
                .iter()
                .any(|trusted| origin == trusted)
    }

    pub fn bridge_script_for_origin(&self, origin: &str) -> Option<&'static str> {
        self.is_trusted_origin(origin)
            .then_some(CEFARI_BRIDGE_SCRIPT)
    }

    pub fn bridge_script_for_url(&self, url: &str) -> Option<&'static str> {
        self.bridge_script_for_origin(&origin_from_url(url)?)
    }
}

pub fn origin_from_url(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.eq_ignore_ascii_case("file") {
        return Some(url.to_owned());
    }

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];

    (!authority.is_empty()).then(|| format!("{}://{}", scheme.to_ascii_lowercase(), authority))
}

fn dev_port_from_url(url: &str) -> Option<u16> {
    let origin = origin_from_url(url)?;
    let authority = origin.split_once("://")?.1;
    let port = authority.rsplit_once(':')?.1;
    port.parse().ok()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NavigationSurface {
    MainFrame,
    SubFrame,
    Popup,
    OpenUrlFromTab,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NavigationDecision {
    Allow,
    OpenExternally,
    Deny,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NavigationPolicyDecision {
    pub decision: NavigationDecision,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NavigationPolicy {
    bridge_origin_policy: BridgeOriginPolicy,
}

impl NavigationPolicy {
    pub fn new(bridge_origin_policy: BridgeOriginPolicy) -> Self {
        Self {
            bridge_origin_policy,
        }
    }

    pub fn from_environment() -> Self {
        Self::new(BridgeOriginPolicy::from_environment())
    }

    pub fn decide(&self, surface: NavigationSurface, url: &str) -> NavigationPolicyDecision {
        match surface {
            NavigationSurface::SubFrame => NavigationPolicyDecision::allow("subframe navigation"),
            NavigationSurface::MainFrame => self.decide_main_frame(url),
            NavigationSurface::Popup | NavigationSurface::OpenUrlFromTab => {
                Self::decide_external_surface(url)
            }
        }
    }

    fn decide_main_frame(&self, url: &str) -> NavigationPolicyDecision {
        if self.is_trusted_url(url) {
            return NavigationPolicyDecision::allow("trusted app navigation");
        }

        if is_supported_external_url(url) {
            return NavigationPolicyDecision::open_externally("external main-frame navigation");
        }

        NavigationPolicyDecision::deny("untrusted main-frame navigation")
    }

    fn decide_external_surface(url: &str) -> NavigationPolicyDecision {
        if is_supported_external_url(url) {
            NavigationPolicyDecision::open_externally("external URL surface")
        } else {
            NavigationPolicyDecision::deny("unsupported external URL")
        }
    }

    fn is_trusted_url(&self, url: &str) -> bool {
        origin_from_url(url)
            .as_deref()
            .is_some_and(|origin| self.bridge_origin_policy.is_trusted_origin(origin))
    }
}

impl NavigationPolicyDecision {
    fn allow(reason: &'static str) -> Self {
        Self {
            decision: NavigationDecision::Allow,
            reason,
        }
    }

    fn open_externally(reason: &'static str) -> Self {
        Self {
            decision: NavigationDecision::OpenExternally,
            reason,
        }
    }

    fn deny(reason: &'static str) -> Self {
        Self {
            decision: NavigationDecision::Deny,
            reason,
        }
    }
}

fn is_supported_external_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")
}

pub struct CefariBridge {
    origin_policy: BridgeOriginPolicy,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DaemonStreamRequest {
    pub id: String,
    pub daemon: DaemonStreamCommand,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum DaemonStreamCommand {
    Connect,
    Write {
        #[serde(rename = "connectionId")]
        connection_id: u64,
        #[serde(rename = "chunkBase64")]
        chunk_base64: String,
    },
    CloseWrite {
        #[serde(rename = "connectionId")]
        connection_id: u64,
    },
    Close {
        #[serde(rename = "connectionId")]
        connection_id: u64,
    },
}

pub fn daemon_stream_request(request_json: &str) -> Option<Result<DaemonStreamRequest, String>> {
    let value = serde_json::from_str::<Value>(request_json).ok()?;
    value.get("daemon")?;
    Some(
        serde_json::from_value::<DaemonStreamRequest>(value)
            .map_err(|error| format!("invalid daemon stream request: {error}")),
    )
}

pub fn daemon_stream_ok_response(id: &str, payload: Value) -> String {
    serde_json::json!({
        "id": id,
        "outcome": {
            "status": "ok",
            "payload": payload,
        },
    })
    .to_string()
}

pub fn daemon_stream_error_response(id: &str, code: &str, message: &str) -> String {
    let payload = if code == "unsupported" {
        serde_json::json!({
            "code": code,
            "details": {
                "command": "daemon",
                "reason": message,
            },
        })
    } else {
        serde_json::json!({
            "code": code,
            "details": {
                "message": message,
            },
        })
    };
    serde_json::json!({
        "id": id,
        "outcome": {
            "status": "err",
            "payload": payload,
        },
    })
    .to_string()
}

impl CefariBridge {
    pub fn new(origin_policy: BridgeOriginPolicy) -> Self {
        Self { origin_policy }
    }

    pub fn handle_json_request(
        &self,
        origin: &str,
        request_json: &str,
        context: &mut impl DesktopIpcContext,
    ) -> String {
        let request_id = request_id(request_json);

        if !self.origin_policy.is_trusted_origin(origin) {
            let response =
                denied_response(request_id, "origin is not allowed to use the Cefari bridge");
            return response_json(&response);
        }

        let request = match serde_json::from_str::<CefariIpcRequest>(request_json) {
            Ok(request) => request,
            Err(error) => {
                let response = parse_error_response(request_id, request_json, &error);
                return response_json(&response);
            }
        };

        response_json(&DesktopIpcDispatcher::dispatch(request, context))
    }
}

pub fn request_id(request_json: &str) -> String {
    serde_json::from_str::<Value>(request_json)
        .ok()
        .and_then(|value| value.get("id").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| "cefari.bridge".to_owned())
}

fn parse_error_response(
    id: String,
    request_json: &str,
    error: &serde_json::Error,
) -> CefariIpcResponse {
    if let Some(command) = command_name(request_json) {
        unknown_command_response(id, &command)
    } else {
        invalid_response(id, &format!("invalid IPC request: {error}"))
    }
}

fn command_name(request_json: &str) -> Option<String> {
    serde_json::from_str::<Value>(request_json)
        .ok()
        .and_then(|value| {
            value
                .get("command")
                .and_then(|command| command.get("command"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

fn denied_response(id: String, message: &str) -> CefariIpcResponse {
    CefariIpcResponse {
        id,
        outcome: CefariIpcOutcome::Err(CefariIpcError::Denied {
            message: message.to_owned(),
        }),
    }
}

pub fn denied_response_json(request_json: &str, message: &str) -> String {
    response_json(&denied_response(request_id(request_json), message))
}

pub fn transport_error_response_json(request_json: &str, reason: &str) -> String {
    response_json(&CefariIpcResponse {
        id: request_id(request_json),
        outcome: CefariIpcOutcome::Err(CefariIpcError::Unsupported {
            command: "bridge".to_owned(),
            reason: reason.to_owned(),
        }),
    })
}

fn invalid_response(id: String, message: &str) -> CefariIpcResponse {
    CefariIpcResponse {
        id,
        outcome: CefariIpcOutcome::Err(CefariIpcError::InvalidCommand {
            message: message.to_owned(),
        }),
    }
}

fn unknown_command_response(id: String, command: &str) -> CefariIpcResponse {
    CefariIpcResponse {
        id,
        outcome: CefariIpcOutcome::Err(CefariIpcError::UnknownCommand {
            command: command.to_owned(),
        }),
    }
}

fn response_json(response: &CefariIpcResponse) -> String {
    serde_json::to_string(&response).expect("IPC responses should serialize")
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use cefari_core::{
        AppDataDirInfo, CefariIpcCommand, CefariIpcError, CefariIpcOutcome, CefariIpcRequest,
        DialogCommand, DialogResult, DownloadCommand, DownloadIdResult, DownloadResult, FileResult,
        FilesCommand, NotificationCommand, NotificationResult, ServiceStatusResult, TrayResult,
        UpdateApplyResult, UpdateCheckResult, UpdateStateKind, UpdateStateResult,
        WindowCreateRequest, WindowKind, WindowListResult, WindowSetTitleRequest, WindowState,
        WindowTargetRequest, WorkerCommand, WorkerListResult, WorkerResult, WorkerState,
        WorkerStatus,
    };

    use super::{
        BridgeOriginPolicy, CEFARI_BRIDGE_SCRIPT, CEFARI_DEFAULT_STYLES, CefariBridge,
        DaemonStreamCommand, NavigationDecision, NavigationPolicy, NavigationSurface,
        daemon_stream_error_response, daemon_stream_ok_response, daemon_stream_request,
        origin_from_url, transport_error_response_json,
    };
    use crate::desktop_ipc;

    #[derive(Debug, Default)]
    struct FakeShellContext {
        update_state_calls: usize,
    }

    impl desktop_ipc::app::AppContext for FakeShellContext {
        fn quit_app(&mut self) -> Result<()> {
            Ok(())
        }
    }

    impl desktop_ipc::windows::WindowContext for FakeShellContext {
        fn window_current(&mut self) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_list(&mut self) -> Result<WindowListResult> {
            Ok(WindowListResult {
                windows: vec![window_state()],
            })
        }

        fn window_create(&mut self, _request: &WindowCreateRequest) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_show(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_focus(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_close(&mut self, _request: &WindowTargetRequest) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_set_title(&mut self, _request: &WindowSetTitleRequest) -> Result<WindowState> {
            Ok(window_state())
        }
    }

    impl desktop_ipc::shell::ShellContext for FakeShellContext {
        fn open_logs(&mut self) -> Result<()> {
            Ok(())
        }

        fn reload_ui(&mut self) -> Result<()> {
            Ok(())
        }

        fn open_external_url(&mut self, _url: &str) -> Result<()> {
            Ok(())
        }
    }

    impl desktop_ipc::updates::UpdateContext for FakeShellContext {
        fn update_state(&mut self) -> Result<UpdateStateResult> {
            self.update_state_calls += 1;
            Ok(UpdateStateResult {
                state: UpdateStateKind::Current,
            })
        }

        fn update_check(&mut self) -> Result<UpdateCheckResult> {
            Ok(UpdateCheckResult {
                state: UpdateStateKind::Current,
                version: None,
                update_id: None,
            })
        }

        fn update_apply(&mut self, _update_id: Option<&str>) -> Result<UpdateApplyResult> {
            Ok(UpdateApplyResult {
                state: UpdateStateKind::ReadyToRestart,
                version: Some("1.2.3".to_owned()),
                restart_required: true,
            })
        }

        fn update_restart(&mut self) -> Result<()> {
            Ok(())
        }
    }

    impl desktop_ipc::service::ServiceContext for FakeShellContext {
        fn service_status(&mut self) -> Result<ServiceStatusResult> {
            Ok(ServiceStatusResult {
                status: "unknown".to_owned(),
            })
        }
    }

    impl desktop_ipc::tray::TrayContext for FakeShellContext {
        fn tray_restore_window(&mut self) -> Result<TrayResult> {
            Ok(TrayResult { restored: true })
        }
    }

    impl desktop_ipc::dialogs::DialogContext for FakeShellContext {
        fn dialog(&mut self, _command: &DialogCommand) -> Result<DialogResult> {
            Ok(DialogResult::Canceled)
        }
    }

    impl desktop_ipc::downloads::DownloadContext for FakeShellContext {
        fn download(&mut self, command: &DownloadCommand) -> Result<DownloadResult> {
            let id = match command {
                DownloadCommand::Cancel(request) | DownloadCommand::Reveal(request) => {
                    request.id.clone()
                }
            };
            Ok(DownloadResult::Canceled(DownloadIdResult { id }))
        }
    }

    impl desktop_ipc::notifications::NotificationContext for FakeShellContext {
        fn notification(
            &mut self,
            command: &NotificationCommand,
        ) -> Result<NotificationResult, CefariIpcError> {
            Err(crate::desktop_ipc::unsupported_notification(
                command,
                "desktop notifications are not available",
            ))
        }
    }

    impl desktop_ipc::files::FilesContext for FakeShellContext {
        fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
            match command {
                FilesCommand::AppDataDir => Ok(FileResult::AppDataDir(AppDataDirInfo {
                    root_kind: "appData".to_owned(),
                    display_path: "/tmp/cefari".to_owned(),
                })),
                FilesCommand::Exists(_) => Ok(FileResult::Exists { exists: true }),
                _ => anyhow::bail!("unsupported test file command"),
            }
        }
    }

    impl desktop_ipc::workers::WorkersContext for FakeShellContext {
        fn worker(&mut self, command: &WorkerCommand) -> Result<WorkerResult, CefariIpcError> {
            match command {
                WorkerCommand::List => Ok(WorkerResult::List(WorkerListResult {
                    workers: vec![WorkerState {
                        id: "worker-1".to_owned(),
                        worker: "thumbnailer".to_owned(),
                        status: WorkerStatus::Running,
                    }],
                })),
                _ => Err(crate::desktop_ipc::workers::unsupported_worker(
                    command,
                    "workers are not available in this test context",
                )),
            }
        }
    }

    fn window_state() -> WindowState {
        WindowState {
            id: "main".to_owned(),
            kind: WindowKind::Main,
            visible: true,
            focused: true,
            title: "Cefari".to_owned(),
            modal: false,
            parent_id: None,
            route: None,
        }
    }

    #[test]
    fn origin_policy_allows_packaged_and_configured_dev_origins() {
        let policy = BridgeOriginPolicy::for_dev_port(5173);

        assert!(policy.is_trusted_origin("cefari://app"));
        assert!(!policy.is_trusted_origin("file:///Applications/Test.app/index.html"));
        assert!(policy.is_trusted_origin("http://127.0.0.1:5173"));
        assert!(!policy.is_trusted_origin("http://127.0.0.1:5174"));
        assert!(!policy.is_trusted_origin("https://example.test"));
    }

    #[test]
    fn bridge_script_is_only_returned_for_trusted_origins() {
        let policy = BridgeOriginPolicy::for_dev_port(5173);

        assert_eq!(
            policy.bridge_script_for_origin("http://127.0.0.1:5173"),
            Some(CEFARI_BRIDGE_SCRIPT)
        );
        assert_eq!(
            policy.bridge_script_for_origin("https://example.test"),
            None
        );
        assert!(CEFARI_BRIDGE_SCRIPT.contains("window.cefari"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("window.__CEFARI_IPC_QUERY__"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("\"__CEFARI_IPC_POST__\""));
    }

    #[test]
    fn origin_policy_maps_page_urls_before_returning_bridge_script() {
        let policy = BridgeOriginPolicy::for_dev_port(5173);

        assert_eq!(
            origin_from_url("http://127.0.0.1:5173/dashboard").as_deref(),
            Some("http://127.0.0.1:5173")
        );
        assert_eq!(
            origin_from_url("cefari://app/index.html").as_deref(),
            Some("cefari://app")
        );
        assert!(origin_from_url("not a url").is_none());
        assert_eq!(
            policy.bridge_script_for_url("http://127.0.0.1:5173/dashboard"),
            Some(CEFARI_BRIDGE_SCRIPT)
        );
        assert_eq!(
            policy.bridge_script_for_url("https://example.test/dashboard"),
            None
        );
    }

    #[test]
    fn bridge_script_injects_default_drag_region_styles() {
        assert!(CEFARI_DEFAULT_STYLES.contains(".cefari-drag"));
        assert!(CEFARI_DEFAULT_STYLES.contains(".cefari-no-drag"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("cefari-default-styles"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("-webkit-app-region: drag"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("-webkit-app-region: no-drag"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains(".cefari-drag button"));
        assert!(!CEFARI_BRIDGE_SCRIPT.contains("header {"));
        assert!(!CEFARI_BRIDGE_SCRIPT.contains("nav {"));
    }

    #[test]
    fn bridge_script_installs_daemon_stream_hooks() {
        assert!(CEFARI_BRIDGE_SCRIPT.contains("__CEFARI_DAEMON_STREAM_POST__"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("__CEFARI_DAEMON_STREAM_ON__"));
        assert!(CEFARI_BRIDGE_SCRIPT.contains("__CEFARI_DAEMON_STREAM_EVENT__"));
    }

    #[test]
    fn parses_daemon_stream_requests_separately_from_ipc() {
        let request = daemon_stream_request(
            r#"{"id":"daemon-1","daemon":{"op":"write","connectionId":7,"chunkBase64":"cGluZw=="}}"#,
        )
        .expect("request should be detected")
        .expect("request should parse");

        assert_eq!(request.id, "daemon-1");
        assert_eq!(
            request.daemon,
            DaemonStreamCommand::Write {
                connection_id: 7,
                chunk_base64: "cGluZw==".to_owned(),
            }
        );
        assert!(
            daemon_stream_request(r#"{"id":"ipc-1","command":{"command":"appQuit"}}"#).is_none()
        );
    }

    #[test]
    fn daemon_stream_response_helpers_use_outcome_shape() {
        let ok: serde_json::Value = serde_json::from_str(&daemon_stream_ok_response(
            "daemon-1",
            serde_json::json!({"connectionId": 1}),
        ))
        .expect("response should parse");
        assert_eq!(ok["id"], "daemon-1");
        assert_eq!(ok["outcome"]["status"], "ok");
        assert_eq!(ok["outcome"]["payload"]["connectionId"], 1);

        let error: serde_json::Value = serde_json::from_str(&daemon_stream_error_response(
            "daemon-1",
            "unsupported",
            "daemon is not configured",
        ))
        .expect("response should parse");
        assert_eq!(error["id"], "daemon-1");
        assert_eq!(error["outcome"]["status"], "err");
        assert_eq!(error["outcome"]["payload"]["code"], "unsupported");
        assert_eq!(
            error["outcome"]["payload"]["details"]["reason"],
            "daemon is not configured"
        );
        assert_eq!(error["outcome"]["payload"]["details"]["command"], "daemon");
    }

    #[test]
    fn transport_errors_return_typed_unsupported_response() {
        let request = CefariIpcRequest {
            id: "request-transport".to_owned(),
            command: CefariIpcCommand::UpdateState,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");

        let response_json =
            transport_error_response_json(&request_json, "native IPC transport failed");
        let response = serde_json::from_str::<cefari_core::CefariIpcResponse>(&response_json)
            .expect("response should deserialize");

        assert_eq!(response.id, "request-transport");
        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::Unsupported { command, reason })
                if command == "bridge" && reason == "native IPC transport failed"
        ));
    }

    #[test]
    fn navigation_policy_allows_trusted_main_frame_loads() {
        let policy = NavigationPolicy::new(BridgeOriginPolicy::for_dev_port(5173));

        let decision = policy.decide(NavigationSurface::MainFrame, "http://127.0.0.1:5173");

        assert_eq!(decision.decision, NavigationDecision::Allow);
    }

    #[test]
    fn navigation_policy_opens_external_main_frame_links_outside_cef() {
        let policy = NavigationPolicy::new(BridgeOriginPolicy::for_dev_port(5173));

        let decision = policy.decide(NavigationSurface::MainFrame, "https://example.test/docs");

        assert_eq!(decision.decision, NavigationDecision::OpenExternally);
    }

    #[test]
    fn navigation_policy_denies_unsupported_main_frame_schemes() {
        let policy = NavigationPolicy::new(BridgeOriginPolicy::for_dev_port(5173));

        let decision = policy.decide(NavigationSurface::MainFrame, "custom://example/path");

        assert_eq!(decision.decision, NavigationDecision::Deny);
    }

    #[test]
    fn navigation_policy_opens_supported_popups_externally() {
        let policy = NavigationPolicy::new(BridgeOriginPolicy::for_dev_port(5173));

        let decision = policy.decide(NavigationSurface::Popup, "mailto:hello@example.test");

        assert_eq!(decision.decision, NavigationDecision::OpenExternally);
    }

    #[test]
    fn navigation_policy_allows_subframe_navigation_without_bridge_trust() {
        let policy = NavigationPolicy::new(BridgeOriginPolicy::for_dev_port(5173));

        let decision = policy.decide(NavigationSurface::SubFrame, "https://example.test/frame");

        assert_eq!(decision.decision, NavigationDecision::Allow);
    }

    #[test]
    fn trusted_origin_invokes_harmless_dispatcher_command() {
        let bridge = CefariBridge::new(BridgeOriginPolicy::for_dev_port(5173));
        let request = CefariIpcRequest {
            id: "request-1".to_owned(),
            command: CefariIpcCommand::UpdateState,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");
        let mut context = FakeShellContext::default();

        let response_json =
            bridge.handle_json_request("http://127.0.0.1:5173", &request_json, &mut context);
        let response = serde_json::from_str::<cefari_core::CefariIpcResponse>(&response_json)
            .expect("response should deserialize");

        assert_eq!(context.update_state_calls, 1);
        assert!(matches!(response.outcome, CefariIpcOutcome::Ok(_)));
    }

    #[test]
    fn untrusted_origin_receives_typed_denied_error() {
        let bridge = CefariBridge::new(BridgeOriginPolicy::for_dev_port(5173));
        let request = CefariIpcRequest {
            id: "request-2".to_owned(),
            command: CefariIpcCommand::UpdateState,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");
        let mut context = FakeShellContext::default();

        let response_json =
            bridge.handle_json_request("https://example.test", &request_json, &mut context);
        let response = serde_json::from_str::<cefari_core::CefariIpcResponse>(&response_json)
            .expect("response should deserialize");

        assert_eq!(context.update_state_calls, 0);
        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::Denied { .. })
        ));
    }

    #[test]
    fn transport_denial_preserves_request_id_without_dispatch() {
        let request = CefariIpcRequest {
            id: "request-transport-denied".to_owned(),
            command: CefariIpcCommand::UpdateState,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");

        let response_json = super::denied_response_json(&request_json, "origin is not allowed");
        let response = serde_json::from_str::<cefari_core::CefariIpcResponse>(&response_json)
            .expect("response should deserialize");

        assert_eq!(response.id, "request-transport-denied");
        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::Denied { .. })
        ));
    }

    #[test]
    fn unknown_commands_return_typed_unknown_error() {
        let bridge = CefariBridge::new(BridgeOriginPolicy::for_dev_port(5173));
        let mut context = FakeShellContext::default();

        let response_json = bridge.handle_json_request(
            "http://127.0.0.1:5173",
            r#"{"id":"request-3","command":{"command":"rawShell","payload":{"program":"sh"}}}"#,
            &mut context,
        );
        let response = serde_json::from_str::<cefari_core::CefariIpcResponse>(&response_json)
            .expect("response should deserialize");

        assert!(matches!(
            response.outcome,
            CefariIpcOutcome::Err(CefariIpcError::UnknownCommand { .. })
        ));
    }
}
