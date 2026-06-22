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
        let thread_running = running.clone();
        let thread = thread::spawn(move || run_mux(listener, state, thread_running));
        info!(
            public = %config.public_endpoint.socket_addr(),
            cef = %config.cef_endpoint.socket_addr(),
            "started Cefari DevTools mux"
        );
        Ok(Self {
            endpoint: config.public_endpoint,
            running,
            thread: Some(thread),
        })
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
}

impl MuxState {
    fn new(config: DevtoolsSessionConfig) -> Self {
        Self {
            config,
            routes: Mutex::new(BTreeMap::new()),
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

    fn public_ws_url(&self, id: &str) -> String {
        format!(
            "ws://{}:{}/cef/{id}",
            self.config.public_endpoint.host,
            self.config.public_endpoint.port.get()
        )
    }
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
                let state = state.clone();
                thread::spawn(move || {
                    if let Err(error) = handle_mux_connection(stream, state) {
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
    write_http_response(&mut stream, 404, "Not Found", "text/plain", b"not found")
}

fn proxied_json_version(state: &MuxState) -> Result<String> {
    let body = http_get_body(state.config.cef_endpoint, "/json/version")?;
    let mut value: serde_json::Value =
        serde_json::from_str(&body).context("CEF /json/version returned invalid JSON")?;
    rewrite_target_ws_url(state, "browser", &mut value)?;
    serde_json::to_string(&value).context("failed to serialize proxied /json/version")
}

fn proxied_json_list(state: &MuxState) -> Result<String> {
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
    serde_json::to_string(&value).context("failed to serialize proxied /json/list")
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
        net::TcpListener,
    };

    use super::{
        BackendWsUrl, DEVTOOLS_LOOPBACK_HOST, DevtoolsEndpoint, DevtoolsEndpointRole, DevtoolsPort,
        MuxState, allocate_private_loopback_endpoint, proxied_json_list,
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
    fn rewrites_cef_json_list_websocket_urls_to_mux_routes() {
        let cef_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let cef_port = cef_listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = cef_listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 128];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            let body = format!(
                r#"[{{"id":"page-1","type":"page","webSocketDebuggerUrl":"ws://127.0.0.1:{cef_port}/devtools/page/page-1"}}]"#
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let public =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));
        let cef =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PrivateCef, DevtoolsPort(cef_port));
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
}
