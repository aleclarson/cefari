use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
use cefari_core::{
    AppIdentity, CEFARI_DAEMON_LOG_ENV, LogRotation, RuntimeLogConfig, RuntimePaths,
    prune_rotated_logs,
};
use serde::Serialize;

use crate::{
    build::{workspace_manifest, workspace_target_dir},
    project::{FrontendConfig, ProjectConfig},
};

#[cfg(target_os = "macos")]
const MACOS_DEV_APP_EXECUTABLE: &str = "cefari-desktop";
#[cfg(target_os = "macos")]
const MACOS_DEV_APP_BUNDLE_IDENTIFIER: &str = "dev.cefari.app";
const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
const CEFARI_DEVTOOLS_PORT_ENV: &str = "CEFARI_DEVTOOLS_PORT";

pub fn dev_project(
    project_dir: &Path,
    frontend_port: Option<u16>,
    devtools_port: Option<u16>,
) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let frontend_port = frontend_port.unwrap_or(project.frontend.dev_port);
    let devtools = DevtoolsEndpoint::for_port(resolve_devtools_port(devtools_port)?)?;

    let mut processes = DevProcesses::new();
    processes.start_frontend(project_dir, &project.frontend, frontend_port)?;
    devtools.write_project_file(project_dir)?;
    println!("chrome devtools: {}", devtools.browser_url);
    println!(
        "chrome-devtools start --browserUrl {}",
        devtools.browser_url
    );
    processes.spawn_daemon(project_dir, &project)?;
    processes.spawn_desktop(project_dir, &devtools)?;
    processes.wait()
}

#[derive(Debug, Serialize)]
struct DevtoolsEndpoint {
    port: u16,
    #[serde(rename = "browserUrl")]
    browser_url: String,
}

impl DevtoolsEndpoint {
    fn for_port(port: u16) -> Result<Self> {
        if port == 0 {
            anyhow::bail!("devtools port must be a fixed port");
        }
        Ok(Self {
            port,
            browser_url: format!("http://127.0.0.1:{port}"),
        })
    }

    fn write_project_file(&self, project_dir: &Path) -> Result<()> {
        let devtools_dir = project_dir.join(".cefari");
        fs::create_dir_all(&devtools_dir).with_context(|| {
            format!(
                "failed to create Cefari devtools directory at {}",
                devtools_dir.display()
            )
        })?;
        let devtools_file = devtools_dir.join("devtools.json");
        let json = serde_json::to_string_pretty(self)
            .context("failed to encode Cefari devtools endpoint")?;
        fs::write(&devtools_file, format!("{json}\n")).with_context(|| {
            format!(
                "failed to write Cefari devtools endpoint to {}",
                devtools_file.display()
            )
        })
    }
}

struct DevProcesses {
    frontend: Option<StaticDevServer>,
    frontend_url: Option<String>,
    children: Vec<NamedChild>,
}

impl DevProcesses {
    fn new() -> Self {
        Self {
            frontend: None,
            frontend_url: None,
            children: Vec::new(),
        }
    }

    fn start_frontend(
        &mut self,
        project_dir: &Path,
        frontend: &FrontendConfig,
        port: u16,
    ) -> Result<()> {
        if let Some(command) = &frontend.dev_command {
            let child = spawn_frontend_command(project_dir, command, port)?;
            self.children
                .push(NamedChild::frontend("frontend dev server", child));
            self.frontend_url = Some(format!("http://127.0.0.1:{port}"));
            println!("frontend dev server: http://127.0.0.1:{port}");
            return Ok(());
        }

        let frontend_dir = project_dir.join("frontend");
        let frontend = StaticDevServer::start(&frontend_dir, port)?;
        let frontend_url = format!("http://{}", frontend.address());
        println!("frontend dev server: {frontend_url}");
        self.frontend_url = Some(frontend_url);
        self.frontend = Some(frontend);
        Ok(())
    }

    fn spawn_daemon(&mut self, project_dir: &Path, project: &ProjectConfig) -> Result<()> {
        let log_config = RuntimeLogConfig::new(&RuntimePaths::resolve(&AppIdentity::cefari())?);
        let daemon_log = open_daemon_log(&log_config)?;
        let child = Command::new("deno")
            .arg("run")
            .arg("--watch")
            .arg("--allow-read")
            .arg("--allow-net")
            .arg(&project.daemon.entry)
            .current_dir(project_dir)
            .env(
                CEFARI_DAEMON_LOG_ENV,
                log_config.daemon.file_path().display().to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                daemon_log
                    .try_clone()
                    .context("failed to clone daemon log for stdout")?,
            ))
            .stderr(Stdio::from(daemon_log))
            .spawn()
            .context("failed to start Deno daemon for cefari dev")?;
        self.children.push(NamedChild::new("deno daemon", child));
        Ok(())
    }

    fn spawn_desktop(&mut self, project_dir: &Path, devtools: &DevtoolsEndpoint) -> Result<()> {
        let frontend_url = self.frontend_url.as_deref();
        let mut command = desktop_launch_command()?;
        let child = command
            .envs(frontend_url.map(|url| ("CEFARI_FRONTEND_URL", url)))
            .env(CEFARI_DEV_MODE_ENV, "1")
            .env(CEFARI_DEVTOOLS_PORT_ENV, devtools.port.to_string())
            .env("CEFARI_RESOURCE_DIR", project_dir)
            .stdin(Stdio::null())
            .spawn()
            .context("failed to start Rust desktop app for cefari dev")?;
        self.children
            .push(NamedChild::new("cefari desktop app", child));
        Ok(())
    }

    fn wait(mut self) -> Result<()> {
        loop {
            for index in 0..self.children.len() {
                if let Some(status) = self.children[index].child.try_wait().with_context(|| {
                    format!("failed to poll {}", self.children[index].description)
                })? {
                    let description = self.children[index].description;
                    self.shutdown();

                    if status.success() {
                        println!("{description} exited; stopped cefari dev");
                        return Ok(());
                    }

                    anyhow::bail!("{description} failed with status {status}");
                }
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn shutdown(&mut self) {
        if let Some(frontend) = &mut self.frontend {
            frontend.shutdown();
        }

        for child in self
            .children
            .iter_mut()
            .filter(|child| child.is_frontend_dev_server)
        {
            child.stop();
        }

        for child in self
            .children
            .iter_mut()
            .filter(|child| !child.is_frontend_dev_server)
        {
            child.stop();
        }
    }
}

fn resolve_devtools_port(port: Option<u16>) -> Result<u16> {
    match port {
        Some(0) | None => available_local_port(),
        Some(port) => Ok(port),
    }
}

fn available_local_port() -> Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .context("failed to allocate a local CEF DevTools port")?;
    listener
        .local_addr()
        .context("failed to read allocated CEF DevTools port")
        .map(|address| address.port())
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
fn configure_desktop_run_command(command: &mut Command) {
    command
        .arg("run")
        .arg("--manifest-path")
        .arg(workspace_manifest())
        .arg("-p")
        .arg("cefari-desktop");
}

#[cfg(not(target_os = "macos"))]
fn desktop_launch_command() -> Result<Command> {
    let mut command = Command::new("cargo");
    configure_desktop_run_command(&mut command);
    Ok(command)
}

#[cfg(target_os = "macos")]
fn desktop_launch_command() -> Result<Command> {
    let mut build = Command::new("cargo");
    configure_desktop_build_command(&mut build);
    let status = build
        .status()
        .context("failed to run cargo build for cefari-desktop")?;
    if !status.success() {
        anyhow::bail!("cargo build -p cefari-desktop failed with status {status}");
    }

    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    let desktop_binary = workspace_target_dir(false).join(MACOS_DEV_APP_EXECUTABLE);
    let app_executable = prepare_macos_dev_app(&paths.cache_dir, &desktop_binary)?;
    Ok(Command::new(app_executable))
}

#[cfg(target_os = "macos")]
fn configure_desktop_build_command(command: &mut Command) {
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(workspace_manifest())
        .arg("-p")
        .arg("cefari-desktop");
}

#[cfg(target_os = "macos")]
fn prepare_macos_dev_app(cache_dir: &Path, desktop_binary: &Path) -> Result<PathBuf> {
    let app_path = cache_dir
        .join("dev-app")
        .join(format!("{MACOS_DEV_APP_EXECUTABLE}.app"));
    let contents_dir = app_path.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let frameworks_dir = contents_dir.join("Frameworks");
    let resources_dir = contents_dir.join("Resources");
    let app_executable = macos_dir.join(MACOS_DEV_APP_EXECUTABLE);

    for directory in [&macos_dir, &frameworks_dir, &resources_dir] {
        fs::create_dir_all(directory)
            .with_context(|| format!("failed to create {}", directory.display()))?;
    }
    fs::write(contents_dir.join("Info.plist"), macos_dev_app_info_plist()).with_context(|| {
        format!(
            "failed to write macOS dev app Info.plist under {}",
            contents_dir.display()
        )
    })?;
    fs::copy(desktop_binary, &app_executable).with_context(|| {
        format!(
            "failed to copy desktop executable {} -> {}",
            desktop_binary.display(),
            app_executable.display()
        )
    })?;

    Ok(app_executable)
}

#[cfg(target_os = "macos")]
fn macos_dev_app_info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Cefari Dev</string>
    <key>CFBundleExecutable</key>
    <string>{MACOS_DEV_APP_EXECUTABLE}</string>
    <key>CFBundleIdentifier</key>
    <string>{MACOS_DEV_APP_BUNDLE_IDENTIFIER}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Cefari Dev</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
"#
    )
}

impl Drop for DevProcesses {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct NamedChild {
    description: &'static str,
    child: Child,
    is_frontend_dev_server: bool,
}

impl NamedChild {
    fn new(description: &'static str, child: Child) -> Self {
        Self {
            description,
            child,
            is_frontend_dev_server: false,
        }
    }

    fn frontend(description: &'static str, child: Child) -> Self {
        Self {
            description,
            child,
            is_frontend_dev_server: true,
        }
    }

    fn stop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }

        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_frontend_command(project_dir: &Path, command: &[String], port: u16) -> Result<Child> {
    if command.is_empty() {
        anyhow::bail!("frontend dev_command must contain at least one argument");
    }
    if port == 0 {
        anyhow::bail!(
            "frontend dev_command requires a fixed port; set frontend.dev_port or pass --frontend-port"
        );
    }

    let program = &command[0];
    let args = command[1..]
        .iter()
        .map(|arg| substitute_frontend_port(arg, port))
        .collect::<Vec<_>>();
    Command::new(program)
        .args(args)
        .current_dir(project_dir)
        .env("CEFARI_FRONTEND_PORT", port.to_string())
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to start frontend dev command {program}"))
}

fn open_daemon_log(config: &RuntimeLogConfig) -> Result<File> {
    fs::create_dir_all(&config.directory).with_context(|| {
        format!(
            "failed to create log directory at {}",
            config.directory.display()
        )
    })?;
    rotate_daemon_log(config)?;
    prune_rotated_logs(&config.daemon).with_context(|| {
        format!(
            "failed to prune rotated daemon logs in {}",
            config.directory.display()
        )
    })?;

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.daemon.file_path())
        .with_context(|| {
            format!(
                "failed to open daemon log at {}",
                config.daemon.file_path().display()
            )
        })
}

fn rotate_daemon_log(config: &RuntimeLogConfig) -> Result<()> {
    let LogRotation::Size { max_bytes } = config.daemon.rotation else {
        return Ok(());
    };
    let current = config.daemon.file_path();
    if fs::metadata(&current).map_or(0, |metadata| metadata.len()) < max_bytes {
        return Ok(());
    }

    let oldest = config.directory.join(format!(
        "{}.{}",
        config.daemon.file_name, config.daemon.retained_files
    ));
    if oldest.exists() {
        fs::remove_file(&oldest)
            .with_context(|| format!("failed to remove old daemon log {}", oldest.display()))?;
    }

    for index in (1..config.daemon.retained_files).rev() {
        let from = config
            .directory
            .join(format!("{}.{}", config.daemon.file_name, index));
        if !from.exists() {
            continue;
        }
        let to = config
            .directory
            .join(format!("{}.{}", config.daemon.file_name, index + 1));
        fs::rename(&from, &to).with_context(|| {
            format!(
                "failed to rotate daemon log from {} to {}",
                from.display(),
                to.display()
            )
        })?;
    }

    let first = config
        .directory
        .join(format!("{}.1", config.daemon.file_name));
    fs::rename(&current, &first).with_context(|| {
        format!(
            "failed to rotate daemon log from {} to {}",
            current.display(),
            first.display()
        )
    })?;

    Ok(())
}

fn substitute_frontend_port(arg: &str, port: u16) -> OsString {
    OsString::from(arg.replace("{port}", &port.to_string()))
}

struct StaticDevServer {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl StaticDevServer {
    fn start(frontend_dir: &Path, port: u16) -> Result<Self> {
        if !frontend_dir.join("index.html").exists() {
            anyhow::bail!(
                "missing frontend entry {}; run cefari init or add frontend/index.html",
                frontend_dir.join("index.html").display()
            );
        }

        let listener = TcpListener::bind(("127.0.0.1", port))
            .with_context(|| format!("failed to bind frontend dev server on 127.0.0.1:{port}"))?;
        listener
            .set_nonblocking(true)
            .context("failed to configure frontend dev server socket")?;

        let address = listener
            .local_addr()
            .context("failed to read frontend dev server address")?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let server_shutdown = Arc::clone(&shutdown);
        let frontend_dir = frontend_dir.to_path_buf();
        let handle =
            thread::spawn(move || serve_static_files(listener, frontend_dir, server_shutdown));

        Ok(Self {
            address,
            shutdown,
            handle: Some(handle),
        })
    }

    fn address(&self) -> SocketAddr {
        self.address
    }

    fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StaticDevServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[allow(clippy::needless_pass_by_value)]
fn serve_static_files(listener: TcpListener, frontend_dir: PathBuf, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle_static_request(stream, &frontend_dir);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
}

fn handle_static_request(mut stream: TcpStream, frontend_dir: &Path) -> Result<()> {
    let mut buffer = [0; 1024];
    let bytes = stream
        .read(&mut buffer)
        .context("failed to read frontend dev server request")?;
    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let path = request_path(&request).unwrap_or("/");
    let file = static_file_path(frontend_dir, path);

    match fs::read(&file) {
        Ok(contents) => write_response(&mut stream, "200 OK", mime_type_for_path(&file), &contents),
        Err(_) => write_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
        ),
    }
}

fn request_path(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().nth(1)
}

fn static_file_path(frontend_dir: &Path, request_path: &str) -> PathBuf {
    let request_path = request_path.split('?').next().unwrap_or("/");
    let relative = request_path.trim_start_matches('/');

    if relative.is_empty() || relative.contains("..") {
        frontend_dir.join("index.html")
    } else {
        frontend_dir.join(relative)
    }
}

fn mime_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )
    .context("failed to write frontend dev server response header")?;
    stream
        .write_all(body)
        .context("failed to write frontend dev server response body")?;
    stream
        .flush()
        .context("failed to flush frontend dev server response")?;
    let _ = stream.shutdown(Shutdown::Write);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{ErrorKind, Read, Write},
        thread,
        time::{Duration, Instant},
    };

    use super::{
        StaticDevServer, configure_desktop_run_command, request_path, static_file_path,
        substitute_frontend_port,
    };

    #[test]
    fn extracts_request_path() {
        assert_eq!(
            request_path("GET /assets/app.js HTTP/1.1\r\nhost: localhost\r\n"),
            Some("/assets/app.js")
        );
    }

    #[test]
    fn maps_unsafe_paths_to_index() {
        let frontend = std::path::Path::new("/tmp/frontend");

        assert_eq!(
            static_file_path(frontend, "/../secret"),
            frontend.join("index.html")
        );
    }

    #[test]
    fn substitutes_frontend_port_placeholder() {
        assert_eq!(
            substitute_frontend_port("--port={port}", 5174),
            std::ffi::OsString::from("--port=5174")
        );
    }

    #[test]
    fn desktop_run_command_targets_desktop_crate_without_features() {
        let mut command = std::process::Command::new("cargo");

        configure_desktop_run_command(&mut command);

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|args| args == ["-p", "cefari-desktop"]));
        assert!(!args.iter().any(|arg| arg == "--features"));
    }

    #[test]
    fn serves_frontend_index() {
        let root = temp_dir("frontend-index");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("frontend dir should be created");
        std::fs::write(root.join("index.html"), "hello cefari").expect("index should be written");

        let mut server = StaticDevServer::start(&root, 0).expect("server should start");
        let response = get_until_ok(server.address(), "/");
        assert!(response.contains("200 OK"));
        assert!(response.contains("content-type: text/html; charset=utf-8"));
        assert!(response.contains("hello cefari"));

        server.shutdown();
        std::fs::remove_dir_all(root).expect("frontend dir should be removable");
    }

    #[test]
    fn serves_module_scripts_with_javascript_mime_type() {
        let root = temp_dir("frontend-module");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("frontend dir should be created");
        std::fs::write(root.join("index.html"), "hello cefari").expect("index should be written");
        std::fs::write(root.join("smoke.js"), "export {};").expect("script should be written");

        let mut server = StaticDevServer::start(&root, 0).expect("server should start");
        let response = get_until_ok(server.address(), "/smoke.js");
        assert!(response.contains("200 OK"));
        assert!(response.contains("content-type: text/javascript; charset=utf-8"));
        assert!(response.contains("export {};"));

        server.shutdown();
        std::fs::remove_dir_all(root).expect("frontend dir should be removable");
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-dev-server-test-{label}-{suffix}"))
    }

    fn get_until_ok(address: std::net::SocketAddr, path: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut last_response = String::new();

        while Instant::now() < deadline {
            match get_once(address, path) {
                Ok(response) => {
                    last_response = response;
                    if last_response.contains("200 OK") {
                        return last_response;
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::ConnectionRefused
                            | ErrorKind::ConnectionReset
                            | ErrorKind::TimedOut
                            | ErrorKind::WouldBlock
                    ) => {}
                Err(error) => panic!("response should be readable: {error}"),
            }
            thread::sleep(Duration::from_millis(10));
        }

        last_response
    }

    fn get_once(address: std::net::SocketAddr, path: &str) -> std::io::Result<String> {
        let mut stream = std::net::TcpStream::connect(address)?;
        let request = format!("GET {path} HTTP/1.1\r\nhost: localhost\r\n\r\n");
        stream.write_all(request.as_bytes())?;
        read_response(&mut stream)
    }

    fn read_response(stream: &mut std::net::TcpStream) -> std::io::Result<String> {
        let mut response = Vec::new();
        let mut buffer = [0; 1024];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes) => response.extend_from_slice(&buffer[..bytes]),
                Err(error)
                    if error.kind() == ErrorKind::ConnectionReset && !response.is_empty() =>
                {
                    break;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(String::from_utf8(response).expect("response should be utf-8"))
    }
}
