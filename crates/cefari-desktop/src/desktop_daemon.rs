use std::{
    ffi::OsString,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
};

use anyhow::{Context, Result};
use cefari_core::{DAEMON_LOG_SCOPE, LOG_PROPERTY_CONNECTION_ID, RuntimePaths};

use crate::logging;

pub const CEFARI_DAEMON_ENV: &str = "CEFARI_DAEMON";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DaemonConnectionId(u64);

impl DaemonConnectionId {
    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    fn next(next_id: &mut u64) -> Self {
        let id = Self(*next_id);
        *next_id += 1;
        id
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DaemonEvent {
    Chunk {
        connection_id: DaemonConnectionId,
        bytes: Vec<u8>,
    },
    Closed {
        connection_id: DaemonConnectionId,
    },
    Error {
        connection_id: DaemonConnectionId,
        message: String,
    },
}

pub trait DaemonEventSink: Send + Sync {
    fn send_daemon_event(&self, event: DaemonEvent) -> Result<()>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DaemonProcessConfig {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub working_directory: PathBuf,
    pub environment: Vec<(OsString, OsString)>,
}

pub trait DaemonSpawner: Send + Sync {
    fn spawn_daemon(&self, config: &DaemonProcessConfig) -> Result<DaemonChild>;
}

#[derive(Debug, Default)]
pub struct SystemDaemonSpawner;

impl DaemonSpawner for SystemDaemonSpawner {
    fn spawn_daemon(&self, config: &DaemonProcessConfig) -> Result<DaemonChild> {
        let mut command = Command::new(&config.program);
        command
            .args(&config.args)
            .current_dir(&config.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env(CEFARI_DAEMON_ENV, "1");
        for (key, value) in &config.environment {
            command.env(key, value);
        }

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn daemon at {}", config.program.display()))?;
        let process_id = child.id();
        let stdin = child
            .stdin
            .take()
            .context("spawned daemon did not expose stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("spawned daemon did not expose stdout")?;
        let stderr = child
            .stderr
            .take()
            .context("spawned daemon did not expose stderr")?;

        Ok(DaemonChild::new(
            Box::new(ChildProcess { child }),
            Box::new(stdin),
            Box::new(stdout),
            Box::new(BufReader::new(stderr)),
            Some(process_id),
        ))
    }
}

pub struct DaemonChild {
    process: Box<dyn DaemonChildProcess>,
    stdin: Box<dyn Write + Send>,
    stdout: Box<dyn Read + Send>,
    stderr: Box<dyn BufRead + Send>,
    process_id: Option<u32>,
}

impl DaemonChild {
    pub fn new(
        process: Box<dyn DaemonChildProcess>,
        stdin: Box<dyn Write + Send>,
        stdout: Box<dyn Read + Send>,
        stderr: Box<dyn BufRead + Send>,
        process_id: Option<u32>,
    ) -> Self {
        Self {
            process,
            stdin,
            stdout,
            stderr,
            process_id,
        }
    }
}

pub trait DaemonChildProcess: Send {
    fn kill(&mut self) -> Result<()>;
}

struct ChildProcess {
    child: Child,
}

impl DaemonChildProcess for ChildProcess {
    fn kill(&mut self) -> Result<()> {
        self.child.kill().context("failed to kill daemon process")
    }
}

struct ActiveDaemonConnection {
    id: DaemonConnectionId,
    process: Box<dyn DaemonChildProcess>,
    stdin: Option<Box<dyn Write + Send>>,
}

pub struct DaemonManager {
    config: Option<DaemonProcessConfig>,
    log_router: Arc<logging::LogRouter>,
    spawner: Arc<dyn DaemonSpawner>,
    sink: Arc<dyn DaemonEventSink>,
    next_id: u64,
    active: Option<ActiveDaemonConnection>,
}

impl DaemonManager {
    pub fn new(
        config: Option<DaemonProcessConfig>,
        _paths: RuntimePaths,
        log_router: Arc<logging::LogRouter>,
        spawner: Arc<dyn DaemonSpawner>,
        sink: Arc<dyn DaemonEventSink>,
    ) -> Self {
        Self {
            config,
            log_router,
            spawner,
            sink,
            next_id: 1,
            active: None,
        }
    }

    pub fn connect(&mut self) -> Result<DaemonConnectionId> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("daemon is not configured"))?;
        if self.active.is_some() {
            anyhow::bail!("daemon connection is already active");
        }

        let id = DaemonConnectionId::next(&mut self.next_id);
        let child = self.spawner.spawn_daemon(config)?;
        let stdout = child.stdout;
        let stderr = child.stderr;
        let process_id = child.process_id;
        let sink = self.sink.clone();
        thread::spawn(move || read_daemon_stdout(id, stdout, sink));
        spawn_daemon_stderr_reader(self.log_router.clone(), id, process_id, stderr);
        self.active = Some(ActiveDaemonConnection {
            id,
            process: child.process,
            stdin: Some(child.stdin),
        });

        Ok(id)
    }

    pub fn write(&mut self, id: DaemonConnectionId, bytes: &[u8]) -> Result<()> {
        let active = self.active_for(id)?;
        let stdin = active
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("daemon stdin is closed"))?;
        stdin
            .write_all(bytes)
            .context("failed to write daemon stdin")?;
        stdin.flush().context("failed to flush daemon stdin")
    }

    pub fn close_write(&mut self, id: DaemonConnectionId) -> Result<()> {
        let active = self.active_for(id)?;
        active.stdin = None;
        Ok(())
    }

    pub fn close(&mut self, id: DaemonConnectionId) -> Result<()> {
        let active = self.take_active(id)?;
        let mut process = active.process;
        process.kill()
    }

    pub fn clear_closed(&mut self, id: DaemonConnectionId) {
        if self.active.as_ref().is_some_and(|active| active.id == id) {
            self.active = None;
        }
    }

    fn active_for(&mut self, id: DaemonConnectionId) -> Result<&mut ActiveDaemonConnection> {
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("daemon connection is not active"))?;
        if active.id != id {
            anyhow::bail!("daemon connection id is not active");
        }
        Ok(active)
    }

    fn take_active(&mut self, id: DaemonConnectionId) -> Result<ActiveDaemonConnection> {
        if self.active.as_ref().is_some_and(|active| active.id == id) {
            return Ok(self.active.take().expect("active connection should exist"));
        }
        anyhow::bail!("daemon connection id is not active");
    }
}

fn spawn_daemon_stderr_reader(
    router: Arc<logging::LogRouter>,
    connection_id: DaemonConnectionId,
    process_id: Option<u32>,
    stderr: Box<dyn BufRead + Send>,
) {
    thread::spawn(move || {
        for line in stderr.lines() {
            match line {
                Ok(line) if line.trim().is_empty() => {}
                Ok(line) => {
                    if let Err(error) =
                        append_daemon_stderr_line(&router, connection_id, process_id, &line)
                    {
                        eprintln!("failed to write daemon stderr log: {error}");
                    }
                }
                Err(error) => {
                    eprintln!("failed to read daemon stderr: {error}");
                    break;
                }
            }
        }
    });
}

fn append_daemon_stderr_line(
    router: &logging::LogRouter,
    connection_id: DaemonConnectionId,
    process_id: Option<u32>,
    line: &str,
) -> Result<()> {
    let mut properties = serde_json::json!({
        LOG_PROPERTY_CONNECTION_ID: connection_id.as_u64(),
        "stream": "stderr",
    });
    if let Some(process_id) = process_id {
        properties["childPid"] = serde_json::json!(process_id);
    }

    router.route(&logging::LogEvent::new(
        DAEMON_LOG_SCOPE,
        "log",
        line,
        properties,
    ))
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        if let Some(mut active) = self.active.take() {
            let _ = active.process.kill();
        }
    }
}

fn read_daemon_stdout(
    connection_id: DaemonConnectionId,
    mut stdout: Box<dyn Read + Send>,
    sink: Arc<dyn DaemonEventSink>,
) {
    let mut buffer = [0_u8; 8192];
    loop {
        match stdout.read(&mut buffer) {
            Ok(0) => {
                let _ = sink.send_daemon_event(DaemonEvent::Closed { connection_id });
                return;
            }
            Ok(read) => {
                let _ = sink.send_daemon_event(DaemonEvent::Chunk {
                    connection_id,
                    bytes: buffer[..read].to_vec(),
                });
            }
            Err(error) => {
                let _ = sink.send_daemon_event(DaemonEvent::Error {
                    connection_id,
                    message: error.to_string(),
                });
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Result as IoResult, Write},
        path::PathBuf,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use super::{
        DaemonChild, DaemonChildProcess, DaemonConnectionId, DaemonEvent, DaemonEventSink,
        DaemonManager, DaemonProcessConfig, DaemonSpawner, append_daemon_stderr_line,
    };
    use cefari_core::{AppIdentity, RuntimeLogConfig, RuntimePaths};

    use crate::logging;

    #[test]
    fn connect_rejects_unconfigured_daemon() {
        let mut manager = test_manager(None, Vec::new());

        let error = manager.connect().expect_err("daemon should be absent");

        assert!(error.to_string().contains("daemon is not configured"));
    }

    #[test]
    fn connect_spawns_daemon_and_forwards_stdout() {
        let sink = RecordingSink::default();
        let mut manager = DaemonManager::new(
            Some(test_config()),
            test_paths(),
            test_log_router(),
            Arc::new(FakeSpawner::new(Vec::from("pong"))),
            Arc::new(sink.clone()),
        );

        let id = manager.connect().expect("daemon should connect");

        sink.wait_for_events(2);
        assert_eq!(
            sink.events(),
            vec![
                DaemonEvent::Chunk {
                    connection_id: id,
                    bytes: Vec::from("pong"),
                },
                DaemonEvent::Closed { connection_id: id },
            ]
        );
    }

    #[test]
    fn write_sends_bytes_to_daemon_stdin() {
        let stdin = SharedWriter::default();
        let spawner = Arc::new(FakeSpawner::with_stdin(Vec::new(), stdin.clone()));
        let mut manager = DaemonManager::new(
            Some(test_config()),
            test_paths(),
            test_log_router(),
            spawner,
            Arc::new(RecordingSink::default()),
        );

        let id = manager.connect().expect("daemon should connect");
        manager.write(id, b"ping").expect("write should succeed");

        assert_eq!(stdin.bytes(), b"ping");
    }

    #[test]
    fn close_write_drops_stdin() {
        let mut manager = test_manager(Some(test_config()), Vec::new());
        let id = manager.connect().expect("daemon should connect");

        manager.close_write(id).expect("close write should succeed");
        let error = manager
            .write(id, b"ping")
            .expect_err("stdin should be closed");

        assert!(error.to_string().contains("daemon stdin is closed"));
    }

    #[test]
    fn second_connect_is_rejected_until_closed_is_cleared() {
        let mut manager = test_manager(Some(test_config()), Vec::new());
        let first = manager.connect().expect("first connect should succeed");

        let error = manager.connect().expect_err("second connect should fail");
        assert!(
            error
                .to_string()
                .contains("daemon connection is already active")
        );

        manager.clear_closed(first);
        manager
            .connect()
            .expect("connect should succeed after clear");
    }

    #[test]
    fn drop_kills_active_process() {
        let process = FakeProcess::default();
        let killed = process.killed.clone();
        let spawner = Arc::new(FakeSpawner::with_process(Vec::new(), process));
        {
            let mut manager = DaemonManager::new(
                Some(test_config()),
                test_paths(),
                test_log_router(),
                spawner,
                Arc::new(RecordingSink::default()),
            );
            manager.connect().expect("daemon should connect");
        }

        assert_eq!(*killed.lock().unwrap(), true);
    }

    #[test]
    fn close_kills_active_process() {
        let process = FakeProcess::default();
        let killed = process.killed.clone();
        let spawner = Arc::new(FakeSpawner::with_process(Vec::new(), process));
        let mut manager = DaemonManager::new(
            Some(test_config()),
            test_paths(),
            test_log_router(),
            spawner,
            Arc::new(RecordingSink::default()),
        );
        let id = manager.connect().expect("daemon should connect");

        manager.close(id).expect("close should kill process");

        assert_eq!(*killed.lock().unwrap(), true);
    }

    fn test_manager(config: Option<DaemonProcessConfig>, stdout: Vec<u8>) -> DaemonManager {
        DaemonManager::new(
            config,
            test_paths(),
            test_log_router(),
            Arc::new(FakeSpawner::new(stdout)),
            Arc::new(RecordingSink::default()),
        )
    }

    fn test_paths() -> RuntimePaths {
        RuntimePaths::resolve(&AppIdentity::cefari()).expect("runtime paths should resolve")
    }

    fn test_log_router() -> Arc<logging::LogRouter> {
        Arc::new(
            logging::LogRouter::with_local_database(
                &RuntimeLogConfig::new(&test_paths()).database.file_path(),
            )
            .expect("router should open"),
        )
    }

    fn test_config() -> DaemonProcessConfig {
        DaemonProcessConfig {
            program: PathBuf::from("/tmp/daemon"),
            args: Vec::new(),
            working_directory: PathBuf::from("/tmp"),
            environment: Vec::new(),
        }
    }

    #[test]
    fn appends_daemon_stderr_lines_to_daemon_scope() {
        let root = std::env::temp_dir().join(format!("cefari-daemon-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp dir should exist");
        let database_path = root.join("cefari.sqlite");

        let router = logging::LogRouter::with_local_database(&database_path).expect("router");

        append_daemon_stderr_line(
            &router,
            DaemonConnectionId::from_u64(7),
            Some(4321),
            "daemon warning",
        )
        .expect("daemon stderr should append");

        let connection = rusqlite::Connection::open(&database_path).expect("database should open");
        let row = connection
            .query_row(
                "SELECT scope, level, message, properties_json FROM log_entries",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .expect("log row should exist");

        assert_eq!(row.0, "daemon");
        assert_eq!(row.1, "log");
        assert_eq!(row.2, "daemon warning");
        let properties: serde_json::Value =
            serde_json::from_str(&row.3).expect("properties should be json");
        assert_eq!(properties["connectionId"], 7);
        assert_eq!(properties["childPid"], 4321);
        assert_eq!(properties["stream"], "stderr");

        std::fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[derive(Clone, Default)]
    struct RecordingSink {
        events: Arc<Mutex<Vec<DaemonEvent>>>,
    }

    impl RecordingSink {
        fn events(&self) -> Vec<DaemonEvent> {
            self.events.lock().unwrap().clone()
        }

        fn wait_for_events(&self, count: usize) {
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if self.events.lock().unwrap().len() >= count {
                    return;
                }
                thread::sleep(Duration::from_millis(10));
            }
            panic!("timed out waiting for daemon events");
        }
    }

    impl DaemonEventSink for RecordingSink {
        fn send_daemon_event(&self, event: DaemonEvent) -> anyhow::Result<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct FakeSpawner {
        stdout: Vec<u8>,
        stdin: SharedWriter,
        process: Option<FakeProcess>,
    }

    impl FakeSpawner {
        fn new(stdout: Vec<u8>) -> Self {
            Self {
                stdout,
                stdin: SharedWriter::default(),
                process: None,
            }
        }

        fn with_stdin(stdout: Vec<u8>, stdin: SharedWriter) -> Self {
            Self {
                stdout,
                stdin,
                process: None,
            }
        }

        fn with_process(stdout: Vec<u8>, process: FakeProcess) -> Self {
            Self {
                stdout,
                stdin: SharedWriter::default(),
                process: Some(process),
            }
        }
    }

    impl DaemonSpawner for FakeSpawner {
        fn spawn_daemon(&self, _config: &DaemonProcessConfig) -> anyhow::Result<DaemonChild> {
            Ok(DaemonChild::new(
                Box::new(self.process.clone().unwrap_or_default()),
                Box::new(self.stdin.clone()),
                Box::new(Cursor::new(self.stdout.clone())),
                Box::new(Cursor::new(Vec::new())),
                None,
            ))
        }
    }

    #[derive(Clone, Default)]
    struct SharedWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl SharedWriter {
        fn bytes(&self) -> Vec<u8> {
            self.bytes.lock().unwrap().clone()
        }
    }

    impl Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
            self.bytes.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> IoResult<()> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct FakeProcess {
        killed: Arc<Mutex<bool>>,
    }

    impl DaemonChildProcess for FakeProcess {
        fn kill(&mut self) -> anyhow::Result<()> {
            *self.killed.lock().unwrap() = true;
            Ok(())
        }
    }
}
