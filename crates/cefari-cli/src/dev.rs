use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
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

use crate::{build::workspace_manifest, project::ProjectConfig};

pub fn dev_project(project_dir: &Path, frontend_port: u16) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let frontend_dir = project_dir.join("frontend");
    let frontend = StaticDevServer::start(&frontend_dir, frontend_port)?;
    println!("frontend dev server: http://{}", frontend.address());

    let mut processes = DevProcesses::new(frontend);
    processes.spawn_daemon(project_dir, &project)?;
    processes.spawn_desktop()?;
    processes.wait()
}

struct DevProcesses {
    frontend: StaticDevServer,
    children: Vec<NamedChild>,
}

impl DevProcesses {
    fn new(frontend: StaticDevServer) -> Self {
        Self {
            frontend,
            children: Vec::new(),
        }
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
        self.frontend.shutdown();
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
        .context("failed to write frontend dev server response body")
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use super::{StaticDevServer, request_path, static_file_path};

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
    fn serves_frontend_index() {
        let root =
            std::env::temp_dir().join(format!("cefari-dev-server-test-{}", std::process::id()));
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
}
