#![allow(dead_code)]

use cefari_core::{CefariIpcError, CefariIpcOutcome, CefariIpcRequest, CefariIpcResponse};
use serde_json::Value;

use crate::desktop_ipc::{DesktopIpcDispatcher, NativeShellContext};

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
        return () => listeners.delete(handler);
      },
    }),
  });

  Object.defineProperty(window, "__CEFARI_IPC_EVENT__", {
    configurable: false,
    enumerable: false,
    writable: false,
    value(event) {
      for (const listener of listeners) listener(event);
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
    pub fn for_dev_port(dev_port: u16) -> Self {
        Self {
            trusted_packaged_origins: vec![
                "cefari://app".to_owned(),
                "app://cefari".to_owned(),
                "file://".to_owned(),
            ],
            allowed_dev_origins: vec![
                format!("http://127.0.0.1:{dev_port}"),
                format!("http://localhost:{dev_port}"),
            ],
        }
    }

    pub fn is_trusted_origin(&self, origin: &str) -> bool {
        self.trusted_packaged_origins.iter().any(|trusted| {
            origin == trusted || trusted == "file://" && origin.starts_with("file://")
        }) || self
            .allowed_dev_origins
            .iter()
            .any(|trusted| origin == trusted)
    }

    pub fn bridge_script_for_origin(&self, origin: &str) -> Option<&'static str> {
        self.is_trusted_origin(origin)
            .then_some(CEFARI_BRIDGE_SCRIPT)
    }
}

pub struct CefariBridge {
    origin_policy: BridgeOriginPolicy,
}

impl CefariBridge {
    pub fn new(origin_policy: BridgeOriginPolicy) -> Self {
        Self { origin_policy }
    }

    pub fn handle_json_request(
        &self,
        origin: &str,
        request_json: &str,
        context: &mut impl NativeShellContext,
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

fn request_id(request_json: &str) -> String {
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
        FileResult, FilesCommand, ServiceStatusResult, TrayResult, UpdateCheckResult,
        UpdateStateKind, UpdateStateResult, WindowState,
    };

    use super::{BridgeOriginPolicy, CEFARI_BRIDGE_SCRIPT, CEFARI_DEFAULT_STYLES, CefariBridge};
    use crate::desktop_ipc::NativeShellContext;

    #[derive(Debug, Default)]
    struct FakeShellContext {
        update_state_calls: usize,
    }

    impl NativeShellContext for FakeShellContext {
        fn quit_app(&mut self) -> Result<()> {
            Ok(())
        }

        fn window_show(&mut self) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_focus(&mut self) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_close(&mut self) -> Result<WindowState> {
            Ok(window_state())
        }

        fn window_set_title(&mut self, _title: &str) -> Result<WindowState> {
            Ok(window_state())
        }

        fn open_logs(&mut self) -> Result<()> {
            Ok(())
        }

        fn open_external_url(&mut self, _url: &str) -> Result<()> {
            Ok(())
        }

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
            })
        }

        fn service_status(&mut self) -> Result<ServiceStatusResult> {
            Ok(ServiceStatusResult {
                status: "unknown".to_owned(),
            })
        }

        fn tray_restore_window(&mut self) -> Result<TrayResult> {
            Ok(TrayResult { restored: true })
        }

        fn files(&mut self, command: &FilesCommand) -> Result<FileResult> {
            match command {
                FilesCommand::AppDataDir => Ok(FileResult::AppDataDir(AppDataDirInfo {
                    root_kind: "appData".to_owned(),
                    display_path: "/tmp/cefari".to_owned(),
                })),
                _ => anyhow::bail!("unsupported test file command"),
            }
        }
    }

    fn window_state() -> WindowState {
        WindowState {
            visible: true,
            focused: true,
            title: "Cefari".to_owned(),
        }
    }

    #[test]
    fn origin_policy_allows_packaged_and_configured_dev_origins() {
        let policy = BridgeOriginPolicy::for_dev_port(5173);

        assert!(policy.is_trusted_origin("cefari://app"));
        assert!(policy.is_trusted_origin("file:///Applications/Test.app/index.html"));
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
