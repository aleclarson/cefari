use std::{
    ffi::OsString,
    fs,
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

use crate::{
    build::workspace_manifest,
    project::{FrontendConfig, ProjectConfig},
};

pub fn dev_project(project_dir: &Path, frontend_port: Option<u16>) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let frontend_port = frontend_port.unwrap_or(project.frontend.dev_port);

    let mut processes = DevProcesses::new();
    processes.start_frontend(project_dir, &project.frontend, frontend_port)?;
    processes.spawn_daemon(project_dir, &project)?;
    processes.spawn_desktop()?;
    processes.wait()
}

struct DevProcesses {
    frontend: Option<StaticDevServer>,
    children: Vec<NamedChild>,
}

impl DevProcesses {
    fn new() -> Self {
        Self {
            frontend: None,
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
                .push(NamedChild::new("frontend dev server", child));
            println!("frontend dev server: http://127.0.0.1:{port}");
            return Ok(());
        }

        let frontend_dir = project_dir.join("frontend");
        let frontend = StaticDevServer::start(&frontend_dir, port)?;
        println!("frontend dev server: http://{}", frontend.address());
        self.frontend = Some(frontend);
        Ok(())
    }

    fn spawn_daemon(&mut self, project_dir: &Path, project: &ProjectConfig) -> Result<()> {
        let child = Command::new("deno")
            .arg("run")
            .arg("--watch")
            .arg("--allow-read")
            .arg("--allow-net")
            .arg(&project.daemon.entry)
            .current_dir(project_dir)
            .stdin(Stdio::null())
            .spawn()
            .context("failed to start Deno daemon for cefari dev")?;
        self.children.push(NamedChild::new("deno daemon", child));
        Ok(())
    }

    fn spawn_desktop(&mut self) -> Result<()> {
        let child = Command::new("cargo")
            .arg("run")
            .arg("--manifest-path")
            .arg(workspace_manifest())
            .arg("-p")
            .arg("cefari-desktop")
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
        for child in &mut self.children {
            child.stop();
        }
    }
}

impl Drop for DevProcesses {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct NamedChild {
    description: &'static str,
    child: Child,
}

impl NamedChild {
    fn new(description: &'static str, child: Child) -> Self {
        Self { description, child }
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
        Ok(contents) => write_response(&mut stream, "200 OK", &contents),
        Err(_) => write_response(&mut stream, "404 Not Found", b"not found"),
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

fn write_response(stream: &mut TcpStream, status: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
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
    use std::io::{Read, Write};

    use super::{StaticDevServer, request_path, static_file_path, substitute_frontend_port};

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
    fn serves_frontend_index() {
        let root = temp_dir("frontend-index");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("frontend dir should be created");
        std::fs::write(root.join("index.html"), "hello cefari").expect("index should be written");

        let mut server = StaticDevServer::start(&root, 0).expect("server should start");
        let mut stream = std::net::TcpStream::connect(server.address())
            .expect("server should accept connections");
        stream
            .write_all(b"GET / HTTP/1.1\r\nhost: localhost\r\n\r\n")
            .expect("request should be sent");

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("response should be readable");
        assert!(response.contains("200 OK"));
        assert!(response.contains("hello cefari"));

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
}
