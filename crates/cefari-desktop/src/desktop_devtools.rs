use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha1_smol::Sha1;
use tracing::{debug, info, warn};

pub(crate) const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
pub(crate) const CEFARI_DEVTOOLS_PORT_ENV: &str = "CEFARI_DEVTOOLS_PORT";
pub(crate) const DEVTOOLS_LOOPBACK_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsPort(u16);

impl DevtoolsPort {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        value
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .map(Self)
    }

    pub(crate) fn get(self) -> u16 {
        self.0
    }

    #[cfg(test)]
    pub(crate) fn from_u16_for_test(port: u16) -> Self {
        Self(port)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum DevtoolsEndpointRole {
    PublicMux,
    PrivateCef,
    PrivateDenoDaemon,
    PrivateDenoWorker,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsEndpoint {
    pub(crate) role: DevtoolsEndpointRole,
    pub(crate) host: Ipv4Addr,
    pub(crate) port: DevtoolsPort,
}

impl DevtoolsEndpoint {
    pub(crate) fn loopback(role: DevtoolsEndpointRole, port: DevtoolsPort) -> Self {
        Self {
            role,
            host: DEVTOOLS_LOOPBACK_HOST,
            port,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn socket_addr(self) -> SocketAddrV4 {
        SocketAddrV4::new(self.host, self.port.get())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsSessionConfig {
    pub(crate) public_endpoint: DevtoolsEndpoint,
    pub(crate) cef_endpoint: DevtoolsEndpoint,
}

impl DevtoolsSessionConfig {
    pub(crate) fn from_environment() -> Result<Option<Self>> {
        if !dev_mode_enabled() {
            return Ok(None);
        }
        let Ok(port) = std::env::var(CEFARI_DEVTOOLS_PORT_ENV) else {
            return Ok(None);
        };
        let Some(port) = DevtoolsPort::parse(&port) else {
            return Ok(None);
        };
        let cef_endpoint = allocate_private_loopback_endpoint(DevtoolsEndpointRole::PrivateCef)?;
        Ok(Some(Self {
            public_endpoint: DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, port),
            cef_endpoint,
        }))
    }
}

pub(crate) fn dev_mode_enabled() -> bool {
    std::env::var(CEFARI_DEV_MODE_ENV).as_deref() == Ok("1")
}

pub(crate) fn allocate_private_loopback_endpoint(
    role: DevtoolsEndpointRole,
) -> Result<DevtoolsEndpoint> {
    let listener = TcpListener::bind(SocketAddrV4::new(DEVTOOLS_LOOPBACK_HOST, 0))
        .context("failed to allocate private DevTools loopback port")?;
    let port = listener
        .local_addr()
        .context("failed to read allocated DevTools loopback port")?
        .port();
    Ok(DevtoolsEndpoint::loopback(role, DevtoolsPort(port)))
}

#[derive(Debug)]
pub(crate) struct DevtoolsMux {
    endpoint: DevtoolsEndpoint,
    registry: DevtoolsTargetRegistry,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DevtoolsMux {
    pub(crate) fn start(config: DevtoolsSessionConfig) -> Result<Self> {
        let listener =
            TcpListener::bind(config.public_endpoint.socket_addr()).with_context(|| {
                format!(
                    "failed to bind DevTools mux at {}",
                    config.public_endpoint.socket_addr()
                )
            })?;
        listener
            .set_nonblocking(true)
            .context("failed to configure DevTools mux listener")?;
        let running = Arc::new(AtomicBool::new(true));
        let state = Arc::new(MuxState::new(config));
        let registry = DevtoolsTargetRegistry {
            state: state.clone(),
        };
        let thread_running = running.clone();
        let thread = thread::spawn(move || run_mux(listener, state, thread_running));
        info!(
            public = %config.public_endpoint.socket_addr(),
            cef = %config.cef_endpoint.socket_addr(),
            "started Cefari DevTools mux"
        );
        Ok(Self {
            endpoint: config.public_endpoint,
            registry,
            running,
            thread: Some(thread),
        })
    }

    pub(crate) fn registry(&self) -> DevtoolsTargetRegistry {
        self.registry.clone()
    }
}

impl Drop for DevtoolsMux {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = TcpStream::connect(self.endpoint.socket_addr());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
struct MuxState {
    config: DevtoolsSessionConfig,
    routes: Mutex<BTreeMap<String, BackendWsUrl>>,
    registered_targets: Mutex<BTreeMap<String, RegisteredDevtoolsTarget>>,
}

impl MuxState {
    fn new(config: DevtoolsSessionConfig) -> Self {
        Self {
            config,
            routes: Mutex::new(BTreeMap::new()),
            registered_targets: Mutex::new(BTreeMap::new()),
        }
    }

    fn route(&self, id: &str) -> Option<BackendWsUrl> {
        self.routes.lock().ok()?.get(id).cloned()
    }

    fn set_route(&self, id: impl Into<String>, url: BackendWsUrl) -> Result<()> {
        self.routes
            .lock()
            .map_err(|error| anyhow!("DevTools mux route lock poisoned: {error}"))?
            .insert(id.into(), url);
        Ok(())
    }

    fn register_target(&self, target: RegisteredDevtoolsTarget) -> Result<()> {
        self.registered_targets
            .lock()
            .map_err(|error| anyhow!("DevTools mux target lock poisoned: {error}"))?
            .insert(target.id.clone(), target);
        Ok(())
    }

    fn unregister_target(&self, id: &str) {
        if let Ok(mut targets) = self.registered_targets.lock() {
            targets.remove(id);
        }
        if let Ok(mut routes) = self.routes.lock() {
            routes.remove(id);
        }
    }

    fn registered_targets(&self) -> Vec<RegisteredDevtoolsTarget> {
        self.registered_targets
            .lock()
            .map(|targets| targets.values().cloned().collect())
            .unwrap_or_default()
    }

    fn public_ws_url(&self, id: &str) -> String {
        format!(
            "ws://{}:{}/cef/{id}",
            self.config.public_endpoint.host,
            self.config.public_endpoint.port.get()
        )
    }

    fn unified_ws_url(&self) -> String {
        format!(
            "ws://{}:{}/unified",
            self.config.public_endpoint.host,
            self.config.public_endpoint.port.get()
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DevtoolsTargetRegistry {
    state: Arc<MuxState>,
}

impl DevtoolsTargetRegistry {
    pub(crate) fn register_deno_daemon(
        &self,
        endpoint: DevtoolsEndpoint,
    ) -> Result<DevtoolsTargetRegistration> {
        let target = RegisteredDevtoolsTarget {
            id: "cefari-daemon".to_owned(),
            title: "Cefari Daemon".to_owned(),
            target_type: "worker".to_owned(),
            url: "cefari://daemon".to_owned(),
            endpoint,
        };
        self.state.register_target(target.clone())?;
        Ok(DevtoolsTargetRegistration {
            registry: self.clone(),
            id: target.id,
        })
    }

    pub(crate) fn register_deno_worker(
        &self,
        id: &str,
        worker: &str,
        endpoint: DevtoolsEndpoint,
    ) -> Result<DevtoolsTargetRegistration> {
        let target = RegisteredDevtoolsTarget {
            id: id.to_owned(),
            title: format!("Cefari Worker: {worker} ({id})"),
            target_type: "worker".to_owned(),
            url: format!("cefari://worker/{worker}/{id}"),
            endpoint,
        };
        self.state.register_target(target.clone())?;
        Ok(DevtoolsTargetRegistration {
            registry: self.clone(),
            id: target.id,
        })
    }

    fn unregister(&self, id: &str) {
        self.state.unregister_target(id);
    }
}

#[derive(Debug)]
pub(crate) struct DevtoolsTargetRegistration {
    registry: DevtoolsTargetRegistry,
    id: String,
}

impl Drop for DevtoolsTargetRegistration {
    fn drop(&mut self) {
        self.registry.unregister(&self.id);
    }
}

#[derive(Debug, Clone)]
struct RegisteredDevtoolsTarget {
    id: String,
    title: String,
    target_type: String,
    url: String,
    endpoint: DevtoolsEndpoint,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BackendWsUrl {
    host: String,
    port: u16,
    path: String,
}

impl BackendWsUrl {
    fn parse(value: &str) -> Option<Self> {
        let rest = value.strip_prefix("ws://")?;
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let (host, port) = authority.rsplit_once(':')?;
        let port = port.parse::<u16>().ok()?;
        Some(Self {
            host: host.to_owned(),
            port,
            path: format!("/{path}"),
        })
    }

    fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn run_mux(listener: TcpListener, state: Arc<MuxState>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let state = state.clone();
                thread::spawn(move || {
                    if let Err(error) = handle_mux_connection(stream, state) {
                        #[cfg(test)]
                        eprintln!("DevTools mux connection failed: {error:?}");
                        debug!(%error, "DevTools mux connection failed");
                    }
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                warn!(%error, "DevTools mux listener failed");
                break;
            }
        }
    }
}

fn handle_mux_connection(mut stream: TcpStream, state: Arc<MuxState>) -> Result<()> {
    let request = read_http_request(&mut stream)?;
    let Some((method, path)) = request_line(&request) else {
        write_http_response(
            &mut stream,
            400,
            "Bad Request",
            "text/plain",
            b"bad request",
        )?;
        return Ok(());
    };
    if method != "GET" {
        write_http_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"method not allowed",
        )?;
        return Ok(());
    }
    if path == "/json/version" {
        let body = proxied_json_version(&state)?;
        write_http_response(&mut stream, 200, "OK", "application/json", body.as_bytes())?;
        return Ok(());
    }
    if path == "/json" || path == "/json/list" {
        let body = proxied_json_list(&state)?;
        write_http_response(&mut stream, 200, "OK", "application/json", body.as_bytes())?;
        return Ok(());
    }
    if let Some(id) = path.strip_prefix("/cef/") {
        let Some(backend) = state.route(id) else {
            write_http_response(
                &mut stream,
                404,
                "Not Found",
                "text/plain",
                b"unknown target",
            )?;
            return Ok(());
        };
        proxy_websocket(stream, request, &backend)?;
        return Ok(());
    }
    if path == "/unified" {
        serve_unified_browser_websocket(stream, request, state)?;
        return Ok(());
    }
    write_http_response(&mut stream, 404, "Not Found", "text/plain", b"not found")
}

fn proxied_json_version(state: &MuxState) -> Result<String> {
    let body = http_get_body(state.config.cef_endpoint, "/json/version")?;
    let mut value: serde_json::Value =
        serde_json::from_str(&body).context("CEF /json/version returned invalid JSON")?;
    value["webSocketDebuggerUrl"] = serde_json::Value::String(state.unified_ws_url());
    serde_json::to_string(&value).context("failed to serialize proxied /json/version")
}

fn proxied_json_list(state: &MuxState) -> Result<String> {
    let value = serde_json::Value::Array(collect_json_targets(state)?);
    serde_json::to_string(&value).context("failed to serialize proxied /json/list")
}

fn collect_json_targets(state: &MuxState) -> Result<Vec<serde_json::Value>> {
    let body = http_get_body(state.config.cef_endpoint, "/json/list")?;
    let mut value: serde_json::Value =
        serde_json::from_str(&body).context("CEF /json/list returned invalid JSON")?;
    let targets = value
        .as_array_mut()
        .ok_or_else(|| anyhow!("CEF /json/list did not return an array"))?;
    for (index, target) in targets.iter_mut().enumerate() {
        let route_id = target
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map_or_else(|| index.to_string(), ToOwned::to_owned);
        rewrite_target_ws_url(state, &route_id, target)?;
    }
    for registered in state.registered_targets() {
        targets.push(registered_json_target(state, &registered)?);
    }
    Ok(targets.clone())
}

fn registered_json_target(
    state: &MuxState,
    target: &RegisteredDevtoolsTarget,
) -> Result<serde_json::Value> {
    let mut value = backend_json_target(target).unwrap_or_else(|| {
        serde_json::json!({
            "id": target.id,
            "title": target.title,
            "type": target.target_type,
            "url": target.url,
        })
    });
    value["id"] = serde_json::Value::String(target.id.clone());
    value["title"] = serde_json::Value::String(target.title.clone());
    value["type"] = serde_json::Value::String(target.target_type.clone());
    value["url"] = serde_json::Value::String(target.url.clone());
    rewrite_target_ws_url(state, &target.id, &mut value)?;
    Ok(value)
}

fn backend_json_target(target: &RegisteredDevtoolsTarget) -> Option<serde_json::Value> {
    let body = http_get_body(target.endpoint, "/json/list").ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&body).ok()?;
    value.as_array()?.first().cloned()
}

fn rewrite_target_ws_url(
    state: &MuxState,
    route_id: &str,
    target: &mut serde_json::Value,
) -> Result<()> {
    let Some(backend_url) = target
        .get("webSocketDebuggerUrl")
        .and_then(serde_json::Value::as_str)
        .and_then(BackendWsUrl::parse)
    else {
        return Ok(());
    };
    state.set_route(route_id, backend_url)?;
    target["webSocketDebuggerUrl"] = serde_json::Value::String(state.public_ws_url(route_id));
    Ok(())
}

fn http_get_body(endpoint: DevtoolsEndpoint, path: &str) -> Result<String> {
    let mut stream = TcpStream::connect(endpoint.socket_addr())
        .with_context(|| format!("failed to connect to DevTools backend at {endpoint:?}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\r\n",
        endpoint.host,
        endpoint.port.get()
    )
    .context("failed to send DevTools backend request")?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .context("failed to read DevTools backend response")?;
    let response =
        String::from_utf8(response).context("DevTools backend response was not UTF-8")?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("DevTools backend response was missing headers"))?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        anyhow::bail!("DevTools backend returned non-200 response: {head}");
    }
    Ok(body.to_owned())
}

fn proxy_websocket(mut client: TcpStream, request: String, backend: &BackendWsUrl) -> Result<()> {
    let mut backend_stream = TcpStream::connect(backend.socket_addr())
        .with_context(|| format!("failed to connect to DevTools backend WebSocket {backend:?}"))?;
    let request = rewrite_websocket_request(&request, backend)?;
    backend_stream
        .write_all(request.as_bytes())
        .context("failed to forward DevTools WebSocket upgrade request")?;
    let response = read_http_request(&mut backend_stream)?;
    client
        .write_all(response.as_bytes())
        .context("failed to forward DevTools WebSocket upgrade response")?;
    if !response.starts_with("HTTP/1.1 101") && !response.starts_with("HTTP/1.0 101") {
        return Ok(());
    }

    let mut client_to_backend_client = client
        .try_clone()
        .context("failed to clone DevTools client socket")?;
    let mut client_to_backend_backend = backend_stream
        .try_clone()
        .context("failed to clone DevTools backend socket")?;
    let client_to_backend = thread::spawn(move || {
        let _ = std::io::copy(
            &mut client_to_backend_client,
            &mut client_to_backend_backend,
        );
        let _ = client_to_backend_backend.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut backend_stream, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let _ = client_to_backend.join();
    Ok(())
}

#[derive(Debug)]
struct BrowserSessionTarget {
    target_id: String,
    backend_writer: Arc<Mutex<TcpStream>>,
}

fn serve_unified_browser_websocket(
    mut client: TcpStream,
    request: String,
    state: Arc<MuxState>,
) -> Result<()> {
    write_websocket_server_handshake(&mut client, &request)?;
    let client_writer = Arc::new(Mutex::new(
        client
            .try_clone()
            .context("failed to clone DevTools browser socket")?,
    ));
    let mut sessions: BTreeMap<String, BrowserSessionTarget> = BTreeMap::new();
    let mut next_session_id = 1_u64;
    while let Some(message) = read_ws_text(&mut client)? {
        let value = serde_json::from_str::<serde_json::Value>(&message)
            .context("DevTools browser message was not JSON")?;
        if let Some(session_id) = value
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
        {
            forward_session_message(&session_id, value, &sessions)?;
            continue;
        }
        handle_browser_cdp_message(
            value,
            state.clone(),
            client_writer.clone(),
            &mut sessions,
            &mut next_session_id,
        )?;
    }
    Ok(())
}

fn handle_browser_cdp_message(
    value: serde_json::Value,
    state: Arc<MuxState>,
    client_writer: Arc<Mutex<TcpStream>>,
    sessions: &mut BTreeMap<String, BrowserSessionTarget>,
    next_session_id: &mut u64,
) -> Result<()> {
    let id = value.get("id").and_then(serde_json::Value::as_i64);
    let method = value
        .get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    match method {
        "Browser.getVersion" => send_browser_result(
            &client_writer,
            id,
            serde_json::json!({
                "protocolVersion": "1.3",
                "product": "Cefari DevTools",
                "revision": "",
                "userAgent": "Cefari DevTools",
                "jsVersion": "",
            }),
        ),
        "Target.setDiscoverTargets" => {
            send_browser_result(&client_writer, id, serde_json::json!({}))?;
            for target in target_infos(&state)? {
                send_browser_event(
                    &client_writer,
                    "Target.targetCreated",
                    serde_json::json!({ "targetInfo": target }),
                )?;
            }
            Ok(())
        }
        "Target.setAutoAttach" | "Target.setAttachToFrames" => {
            send_browser_result(&client_writer, id, serde_json::json!({}))
        }
        "Target.getTargets" => send_browser_result(
            &client_writer,
            id,
            serde_json::json!({ "targetInfos": target_infos(&state)? }),
        ),
        "Target.attachToTarget" => {
            let params = value.get("params").cloned().unwrap_or_default();
            let target_id = params
                .get("targetId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("Target.attachToTarget missing targetId"))?;
            let session_id = format!("cefari-session-{next_session_id}");
            *next_session_id += 1;
            let backend = state
                .route(target_id)
                .ok_or_else(|| anyhow!("unknown DevTools target {target_id}"))?;
            let backend_stream = connect_backend_websocket(&backend)?;
            let backend_writer = Arc::new(Mutex::new(
                backend_stream
                    .try_clone()
                    .context("failed to clone DevTools backend target socket")?,
            ));
            sessions.insert(
                session_id.clone(),
                BrowserSessionTarget {
                    target_id: target_id.to_owned(),
                    backend_writer,
                },
            );
            spawn_backend_reader(session_id.clone(), backend_stream, client_writer.clone());
            let target_info = target_infos(&state)?
                .into_iter()
                .find(|target| {
                    target.get("targetId").and_then(serde_json::Value::as_str) == Some(target_id)
                })
                .unwrap_or_else(|| target_info_from_id(target_id, "worker", target_id));
            send_browser_event(
                &client_writer,
                "Target.attachedToTarget",
                serde_json::json!({
                    "sessionId": session_id,
                    "targetInfo": target_info,
                    "waitingForDebugger": false,
                }),
            )?;
            send_browser_result(
                &client_writer,
                id,
                serde_json::json!({ "sessionId": session_id }),
            )
        }
        "Target.detachFromTarget" => {
            let session_id = value
                .get("params")
                .and_then(|params| params.get("sessionId"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Some(target) = sessions.remove(&session_id) {
                let _ = target
                    .backend_writer
                    .lock()
                    .map(|stream| stream.shutdown(Shutdown::Both));
                send_browser_event(
                    &client_writer,
                    "Target.detachedFromTarget",
                    serde_json::json!({
                        "sessionId": session_id,
                        "targetId": target.target_id,
                    }),
                )?;
            }
            send_browser_result(&client_writer, id, serde_json::json!({}))
        }
        "Target.sendMessageToTarget" => {
            let params = value.get("params").cloned().unwrap_or_default();
            let session_id = params
                .get("sessionId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("Target.sendMessageToTarget missing sessionId"))?;
            let message = params
                .get("message")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| anyhow!("Target.sendMessageToTarget missing message"))?;
            send_to_backend(session_id, message, sessions)?;
            send_browser_result(&client_writer, id, serde_json::json!({}))
        }
        _ => send_browser_result(&client_writer, id, serde_json::json!({})),
    }
}

fn target_infos(state: &MuxState) -> Result<Vec<serde_json::Value>> {
    collect_json_targets(state).map(|targets| {
        targets
            .iter()
            .map(|target| {
                let id = target
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("target");
                let target_type = target
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("worker");
                let title = target
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(id);
                target_info_from_id(id, target_type, title)
            })
            .collect()
    })
}

fn target_info_from_id(id: &str, target_type: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "targetId": id,
        "type": target_type,
        "title": title,
        "url": "",
        "attached": false,
        "canAccessOpener": false,
    })
}

fn forward_session_message(
    session_id: &str,
    mut value: serde_json::Value,
    sessions: &BTreeMap<String, BrowserSessionTarget>,
) -> Result<()> {
    if let serde_json::Value::Object(object) = &mut value {
        object.remove("sessionId");
    }
    send_to_backend(session_id, &serde_json::to_string(&value)?, sessions)
}

fn send_to_backend(
    session_id: &str,
    message: &str,
    sessions: &BTreeMap<String, BrowserSessionTarget>,
) -> Result<()> {
    let target = sessions
        .get(session_id)
        .ok_or_else(|| anyhow!("unknown DevTools session {session_id}"))?;
    let mut backend = target
        .backend_writer
        .lock()
        .map_err(|error| anyhow!("DevTools backend lock poisoned: {error}"))?;
    send_ws_text_masked(&mut backend, message)
}

fn spawn_backend_reader(
    session_id: String,
    mut backend: TcpStream,
    client_writer: Arc<Mutex<TcpStream>>,
) {
    thread::spawn(move || {
        loop {
            let message = match read_ws_text(&mut backend) {
                Ok(Some(message)) => message,
                Ok(None) | Err(_) => break,
            };
            let _ = send_browser_event(
                &client_writer,
                "Target.receivedMessageFromTarget",
                serde_json::json!({
                    "sessionId": session_id,
                    "message": message,
                }),
            );
        }
        let _ = send_browser_event(
            &client_writer,
            "Target.detachedFromTarget",
            serde_json::json!({ "sessionId": session_id }),
        );
    });
}

fn send_browser_result(
    client_writer: &Arc<Mutex<TcpStream>>,
    id: Option<i64>,
    result: serde_json::Value,
) -> Result<()> {
    let Some(id) = id else {
        return Ok(());
    };
    send_browser_message(
        client_writer,
        serde_json::json!({ "id": id, "result": result }),
    )
}

fn send_browser_event(
    client_writer: &Arc<Mutex<TcpStream>>,
    method: &str,
    params: serde_json::Value,
) -> Result<()> {
    send_browser_message(
        client_writer,
        serde_json::json!({ "method": method, "params": params }),
    )
}

fn send_browser_message(
    client_writer: &Arc<Mutex<TcpStream>>,
    message: serde_json::Value,
) -> Result<()> {
    let mut client = client_writer
        .lock()
        .map_err(|error| anyhow!("DevTools browser socket lock poisoned: {error}"))?;
    send_ws_text(&mut client, &serde_json::to_string(&message)?)
}

fn connect_backend_websocket(backend: &BackendWsUrl) -> Result<TcpStream> {
    let mut stream = TcpStream::connect(backend.socket_addr())
        .with_context(|| format!("failed to connect to DevTools target {backend:?}"))?;
    let key = "Y2VmYXJpLWRldnRvb2xzIQ==";
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n",
        backend.path,
        backend.socket_addr()
    )
    .context("failed to send backend WebSocket handshake")?;
    let response = read_http_request(&mut stream)?;
    if !response.starts_with("HTTP/1.1 101") && !response.starts_with("HTTP/1.0 101") {
        anyhow::bail!("backend WebSocket handshake failed: {response}");
    }
    Ok(stream)
}

fn write_websocket_server_handshake(stream: &mut TcpStream, request: &str) -> Result<()> {
    let key = http_header(request, "sec-websocket-key")
        .ok_or_else(|| anyhow!("WebSocket request missing Sec-WebSocket-Key"))?;
    let accept = websocket_accept_key(&key);
    write!(
        stream,
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    )
    .context("failed to write DevTools browser WebSocket handshake")
}

fn websocket_accept_key(key: &str) -> String {
    let mut sha1 = Sha1::new();
    sha1.update(key.as_bytes());
    sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    STANDARD.encode(sha1.digest().bytes())
}

fn http_header(request: &str, name: &str) -> Option<String> {
    request.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name)
            .then(|| value.trim().to_owned())
    })
}

fn read_ws_text(stream: &mut TcpStream) -> Result<Option<String>> {
    let mut header = [0_u8; 2];
    match stream.read_exact(&mut header) {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error).context("failed to read WebSocket frame header"),
    }
    let opcode = header[0] & 0x0f;
    if opcode == 0x8 {
        return Ok(None);
    }
    if opcode != 0x1 {
        anyhow::bail!("unsupported WebSocket opcode {opcode}");
    }
    let masked = header[1] & 0x80 != 0;
    let mut length = u64::from(header[1] & 0x7f);
    if length == 126 {
        let mut extended = [0_u8; 2];
        stream.read_exact(&mut extended)?;
        length = u64::from(u16::from_be_bytes(extended));
    } else if length == 127 {
        let mut extended = [0_u8; 8];
        stream.read_exact(&mut extended)?;
        length = u64::from_be_bytes(extended);
    }
    let mask = if masked {
        let mut mask = [0_u8; 4];
        stream.read_exact(&mut mask)?;
        Some(mask)
    } else {
        None
    };
    let mut payload = vec![0_u8; usize::try_from(length).context("WebSocket frame too large")?];
    stream.read_exact(&mut payload)?;
    if let Some(mask) = mask {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    String::from_utf8(payload)
        .map(Some)
        .context("WebSocket text frame was not UTF-8")
}

fn send_ws_text(stream: &mut TcpStream, message: &str) -> Result<()> {
    send_ws_text_frame(stream, message, None)
}

fn send_ws_text_masked(stream: &mut TcpStream, message: &str) -> Result<()> {
    send_ws_text_frame(stream, message, Some([1, 2, 3, 4]))
}

fn send_ws_text_frame(stream: &mut TcpStream, message: &str, mask: Option<[u8; 4]>) -> Result<()> {
    let bytes = message.as_bytes();
    let mut frame = Vec::with_capacity(bytes.len() + 14);
    frame.push(0x81);
    let mask_bit = mask.map_or(0, |_| 0x80);
    if bytes.len() < 126 {
        frame.push(
            mask_bit
                | u8::try_from(bytes.len()).expect("small WebSocket frame length should fit u8"),
        );
    } else if u16::try_from(bytes.len()).is_ok() {
        frame.push(mask_bit | 126);
        frame.extend_from_slice(
            &u16::try_from(bytes.len())
                .expect("medium WebSocket frame length should fit u16")
                .to_be_bytes(),
        );
    } else {
        frame.push(mask_bit | 127);
        frame.extend_from_slice(
            &u64::try_from(bytes.len())
                .expect("WebSocket frame length should fit u64")
                .to_be_bytes(),
        );
    }
    if let Some(mask) = mask {
        frame.extend_from_slice(&mask);
        frame.extend(
            bytes
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
    } else {
        frame.extend_from_slice(bytes);
    }
    stream
        .write_all(&frame)
        .context("failed to write WebSocket text frame")
}

fn rewrite_websocket_request(request: &str, backend: &BackendWsUrl) -> Result<String> {
    let (_, rest) = request
        .split_once("\r\n")
        .ok_or_else(|| anyhow!("WebSocket request was missing headers"))?;
    let mut rewritten = format!("GET {} HTTP/1.1\r\n", backend.path);
    for line in rest.split("\r\n") {
        if line.is_empty() {
            rewritten.push_str("\r\n");
            break;
        }
        if line.to_ascii_lowercase().starts_with("host:") {
            rewritten.push_str(&format!("Host: {}\r\n", backend.socket_addr()));
        } else {
            rewritten.push_str(line);
            rewritten.push_str("\r\n");
        }
    }
    Ok(rewritten)
}

fn read_http_request(stream: &mut TcpStream) -> Result<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut chunk)
            .context("failed to read DevTools HTTP request")?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > 64 * 1024 {
            anyhow::bail!("DevTools HTTP request exceeded header limit");
        }
    }
    String::from_utf8(buffer).context("DevTools HTTP request was not UTF-8")
}

fn request_line(request: &str) -> Option<(&str, &str)> {
    let line = request.lines().next()?;
    let mut parts = line.split_whitespace();
    Some((parts.next()?, parts.next()?))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .context("failed to write DevTools HTTP response headers")?;
    stream
        .write_all(body)
        .context("failed to write DevTools HTTP response body")
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, mpsc},
    };

    use super::{
        BackendWsUrl, DEVTOOLS_LOOPBACK_HOST, DevtoolsEndpoint, DevtoolsEndpointRole, DevtoolsMux,
        DevtoolsPort, DevtoolsTargetRegistry, MuxState, allocate_private_loopback_endpoint,
        proxied_json_list, read_http_request, read_ws_text, send_ws_text, send_ws_text_masked,
        websocket_accept_key, write_websocket_server_handshake,
    };

    #[test]
    fn parses_nonzero_devtools_ports() {
        assert_eq!(DevtoolsPort::parse("9222"), Some(DevtoolsPort(9222)));
        assert_eq!(DevtoolsPort::parse("0"), None);
        assert_eq!(DevtoolsPort::parse("not-a-port"), None);
    }

    #[test]
    fn builds_loopback_endpoint_for_role() {
        let endpoint =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));

        assert_eq!(endpoint.role, DevtoolsEndpointRole::PublicMux);
        assert_eq!(endpoint.socket_addr().ip(), &DEVTOOLS_LOOPBACK_HOST);
        assert_eq!(endpoint.socket_addr().port(), 9222);
    }

    #[test]
    fn allocates_private_loopback_endpoint() {
        let endpoint =
            allocate_private_loopback_endpoint(DevtoolsEndpointRole::PrivateCef).unwrap();

        assert_eq!(endpoint.role, DevtoolsEndpointRole::PrivateCef);
        assert_eq!(endpoint.host, DEVTOOLS_LOOPBACK_HOST);
        assert_ne!(endpoint.port.get(), 0);
    }

    #[test]
    fn parses_backend_websocket_urls() {
        assert_eq!(
            BackendWsUrl::parse("ws://127.0.0.1:9223/devtools/page/abc"),
            Some(BackendWsUrl {
                host: "127.0.0.1".to_owned(),
                port: 9223,
                path: "/devtools/page/abc".to_owned(),
            })
        );
        assert_eq!(BackendWsUrl::parse("wss://127.0.0.1:9223/page"), None);
        assert_eq!(BackendWsUrl::parse("ws://127.0.0.1/page"), None);
    }

    #[test]
    fn computes_websocket_accept_key() {
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn rewrites_cef_json_list_websocket_urls_to_mux_routes() {
        let (cef, cef_port) = spawn_json_backend(|port| {
            format!(
                r#"[{{"id":"page-1","type":"page","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/devtools/page/page-1"}}]"#
            )
        });
        let public =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));
        let state = MuxState::new(super::DevtoolsSessionConfig {
            public_endpoint: public,
            cef_endpoint: cef,
        });

        let body = proxied_json_list(&state).unwrap();

        assert!(body.contains(r#""webSocketDebuggerUrl":"ws://127.0.0.1:9222/cef/page-1""#));
        assert_eq!(
            state.route("page-1"),
            Some(BackendWsUrl {
                host: "127.0.0.1".to_owned(),
                port: cef_port,
                path: "/devtools/page/page-1".to_owned(),
            })
        );
    }

    #[test]
    fn registered_daemon_target_is_added_and_removed_from_json_list() {
        let (cef, _) = spawn_json_backend(|_| "[]".to_owned());
        let (daemon, daemon_port) = spawn_json_backend(|port| {
            format!(
                r#"[{{"id":"deno-daemon","type":"worker","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/ws/deno-daemon"}}]"#
            )
        });
        let public =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));
        let state = Arc::new(MuxState::new(super::DevtoolsSessionConfig {
            public_endpoint: public,
            cef_endpoint: cef,
        }));
        let registry = DevtoolsTargetRegistry {
            state: state.clone(),
        };

        let registration = registry.register_deno_daemon(daemon).unwrap();
        let body = proxied_json_list(&state).unwrap();

        assert!(body.contains(r#""id":"cefari-daemon""#));
        assert!(body.contains(r#""title":"Cefari Daemon""#));
        assert!(body.contains(r#""webSocketDebuggerUrl":"ws://127.0.0.1:9222/cef/cefari-daemon""#));
        assert_eq!(
            state.route("cefari-daemon"),
            Some(BackendWsUrl {
                host: "127.0.0.1".to_owned(),
                port: daemon_port,
                path: "/ws/deno-daemon".to_owned(),
            })
        );

        drop(registration);

        assert!(state.registered_targets().is_empty());
        assert_eq!(state.route("cefari-daemon"), None);
    }

    #[test]
    fn registered_worker_target_is_added_and_removed_from_json_list() {
        let (cef, _) = spawn_json_backend(|_| "[]".to_owned());
        let (worker, worker_port) = spawn_json_backend(|port| {
            format!(
                r#"[{{"id":"deno-worker","type":"worker","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/ws/deno-worker"}}]"#
            )
        });
        let public =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));
        let state = Arc::new(MuxState::new(super::DevtoolsSessionConfig {
            public_endpoint: public,
            cef_endpoint: cef,
        }));
        let registry = DevtoolsTargetRegistry {
            state: state.clone(),
        };

        let registration = registry
            .register_deno_worker("thumbnailer-1", "thumbnailer", worker)
            .unwrap();
        let body = proxied_json_list(&state).unwrap();

        assert!(body.contains(r#""id":"thumbnailer-1""#));
        assert!(body.contains(r#""title":"Cefari Worker: thumbnailer (thumbnailer-1)""#));
        assert!(body.contains(r#""webSocketDebuggerUrl":"ws://127.0.0.1:9222/cef/thumbnailer-1""#));
        assert_eq!(
            state.route("thumbnailer-1"),
            Some(BackendWsUrl {
                host: "127.0.0.1".to_owned(),
                port: worker_port,
                path: "/ws/deno-worker".to_owned(),
            })
        );

        drop(registration);

        assert!(state.registered_targets().is_empty());
        assert_eq!(state.route("thumbnailer-1"), None);
    }

    #[test]
    fn unified_browser_routes_session_messages_to_backend_target() {
        let (target_endpoint, received_backend_message) = spawn_fake_target_websocket();
        let (cef, _) = spawn_json_backend_n(
            move |_| {
                format!(
                    r#"[{{"id":"page-1","title":"Main","type":"page","webSocketDebuggerUrl":"ws://{}:{}/devtools/page/page-1"}}]"#,
                    target_endpoint.host,
                    target_endpoint.port.get()
                )
            },
            2,
        );
        let public_endpoint =
            allocate_private_loopback_endpoint(DevtoolsEndpointRole::PublicMux).unwrap();
        let mux = DevtoolsMux::start(super::DevtoolsSessionConfig {
            public_endpoint,
            cef_endpoint: cef,
        })
        .unwrap();
        let mut client = TcpStream::connect(public_endpoint.socket_addr()).unwrap();
        write!(
            client,
            "GET /unified HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
            public_endpoint.socket_addr()
        )
        .unwrap();
        let handshake = read_http_request(&mut client).unwrap();
        assert!(handshake.starts_with("HTTP/1.1 101"));

        send_ws_text_masked(
            &mut client,
            r#"{"id":1,"method":"Target.getTargets","params":{}}"#,
        )
        .unwrap();
        let targets = read_ws_text(&mut client).unwrap().unwrap();
        assert!(targets.contains(r#""targetId":"page-1""#));

        send_ws_text_masked(
            &mut client,
            r#"{"id":2,"method":"Target.attachToTarget","params":{"targetId":"page-1","flatten":true}}"#,
        )
        .unwrap();
        let attached = read_ws_text(&mut client).unwrap().unwrap();
        let attached_response = read_ws_text(&mut client).unwrap().unwrap();
        assert!(attached.contains(r#""method":"Target.attachedToTarget""#));
        assert!(attached_response.contains(r#""sessionId":"cefari-session-1""#));

        send_ws_text_masked(
            &mut client,
            r#"{"id":7,"sessionId":"cefari-session-1","method":"Runtime.enable","params":{}}"#,
        )
        .unwrap();
        let forwarded = received_backend_message.recv().unwrap();
        assert_eq!(
            forwarded,
            r#"{"id":7,"method":"Runtime.enable","params":{}}"#
        );
        let routed_response = read_ws_text(&mut client).unwrap().unwrap();
        assert!(routed_response.contains(r#""method":"Target.receivedMessageFromTarget""#));
        assert!(routed_response.contains(r#""sessionId":"cefari-session-1""#));
        assert!(routed_response.contains(r#"\"id\":7"#));

        drop(mux);
    }

    fn spawn_json_backend(
        body: impl FnOnce(u16) -> String + Send + 'static,
    ) -> (DevtoolsEndpoint, u16) {
        spawn_json_backend_n(body, 1)
    }

    fn spawn_json_backend_n(
        body: impl FnOnce(u16) -> String + Send + 'static,
        requests: usize,
    ) -> (DevtoolsEndpoint, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = body(port);
        std::thread::spawn(move || {
            for _ in 0..requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut chunk = [0_u8; 128];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut chunk).unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..read]);
                }
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        (
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PrivateCef, DevtoolsPort(port)),
            port,
        )
    }

    fn spawn_fake_target_websocket() -> (DevtoolsEndpoint, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream).unwrap();
            write_websocket_server_handshake(&mut stream, &request).unwrap();
            let message = read_ws_text(&mut stream).unwrap().unwrap();
            sender.send(message).unwrap();
            send_ws_text(&mut stream, r#"{"id":7,"result":{}}"#).unwrap();
        });
        (
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PrivateCef, DevtoolsPort(port)),
            receiver,
        )
    }
}
