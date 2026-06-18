use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    thread,
};

use anyhow::{Context, Result};
use cefari_core::{
    CefariIpcError, CefariIpcEvent, RuntimePaths, WorkerCommand, WorkerConfig, WorkerEntryConfig,
    WorkerErrorEvent, WorkerEvent, WorkerExitEvent, WorkerListResult, WorkerMessageEvent,
    WorkerPermissionConfig, WorkerResult, WorkerSpawnRequest, WorkerSpawnResult, WorkerState,
    WorkerStatus,
};
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Clone)]
pub(crate) struct DesktopWorkerManager {
    config: WorkerConfig,
    paths: RuntimePaths,
    spawner: Arc<dyn WorkerProcessSpawner>,
    events: Arc<dyn WorkerEventSink>,
    processes: BTreeMap<String, ManagedWorkerProcess>,
    next_id: u64,
}

#[derive(Clone)]
struct ManagedWorkerProcess {
    id: String,
    worker: String,
    child: SharedWorkerChild,
    status: WorkerStatus,
}

type SharedWorkerChild = std::sync::Arc<std::sync::Mutex<Box<dyn WorkerChild>>>;

pub(crate) trait WorkerEventSink: Send + Sync {
    fn send_worker_event(&self, event: CefariIpcEvent) -> Result<()>;
}

pub(crate) trait WorkerProcessSpawner: Send + Sync {
    fn spawn(&self, spec: WorkerProcessSpec) -> Result<Box<dyn WorkerChild>>;
}

pub(crate) trait WorkerChild: Send {
    fn take_stdout(&mut self) -> Option<Box<dyn BufRead + Send>>;
    fn kill(&mut self) -> std::io::Result<()>;
    fn has_exited(&mut self) -> std::io::Result<bool>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct WorkerProcessSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub input: String,
    pub id: String,
    pub worker: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WorkerStdoutEnvelope {
    Message { payload: serde_json::Value },
    Result { payload: serde_json::Value },
    Error { error: WorkerStdoutError },
}

#[derive(Debug, Deserialize)]
struct WorkerStdoutError {
    message: String,
}

impl DesktopWorkerManager {
    pub(crate) fn new(
        config: WorkerConfig,
        paths: RuntimePaths,
        events: Arc<dyn WorkerEventSink>,
    ) -> Self {
        Self::with_spawner(config, paths, events, Arc::new(DenoWorkerProcessSpawner))
    }

    pub(crate) fn with_spawner(
        config: WorkerConfig,
        paths: RuntimePaths,
        events: Arc<dyn WorkerEventSink>,
        spawner: Arc<dyn WorkerProcessSpawner>,
    ) -> Self {
        Self {
            config,
            paths,
            spawner,
            events,
            processes: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub(crate) fn dispatch(
        &mut self,
        command: &WorkerCommand,
    ) -> Result<WorkerResult, CefariIpcError> {
        match command {
            WorkerCommand::Spawn(request) => self.spawn(request),
            WorkerCommand::Terminate(request) => self.terminate(&request.id),
            WorkerCommand::List => Ok(WorkerResult::List(self.list())),
        }
    }

    fn spawn(&mut self, request: &WorkerSpawnRequest) -> Result<WorkerResult, CefariIpcError> {
        let entry = self
            .config
            .entries
            .get(&request.worker)
            .ok_or_else(|| CefariIpcError::InvalidCommand {
                message: format!("worker.spawn: unknown worker {}", request.worker),
            })?
            .clone();
        let id = self.next_worker_id(&request.worker);
        let spec = self
            .process_spec(&id, &request.worker, &entry, &request.input_json)
            .map_err(|error| CefariIpcError::InvalidCommand {
                message: format!("worker.spawn: {error}"),
            })?;
        let mut child = self
            .spawner
            .spawn(spec)
            .map_err(|error| CefariIpcError::Unsupported {
                command: "worker.spawn".to_owned(),
                reason: error.to_string(),
            })?;
        if let Some(stdout) = child.take_stdout() {
            spawn_stdout_reader(
                self.events.clone(),
                id.clone(),
                request.worker.clone(),
                stdout,
            );
        }
        let child = std::sync::Arc::new(std::sync::Mutex::new(child));
        self.processes.insert(
            id.clone(),
            ManagedWorkerProcess {
                id: id.clone(),
                worker: request.worker.clone(),
                child,
                status: WorkerStatus::Running,
            },
        );
        Ok(WorkerResult::Spawned(WorkerSpawnResult {
            id,
            worker: request.worker.clone(),
            status: WorkerStatus::Running,
        }))
    }

    fn terminate(&mut self, id: &str) -> Result<WorkerResult, CefariIpcError> {
        let process = self
            .processes
            .get_mut(id)
            .ok_or_else(|| CefariIpcError::InvalidCommand {
                message: format!("worker.terminate: unknown worker id {id}"),
            })?;
        if matches!(process.status, WorkerStatus::Running) {
            let mut child = process
                .child
                .lock()
                .map_err(|error| CefariIpcError::Unsupported {
                    command: "worker.terminate".to_owned(),
                    reason: format!("worker process lock poisoned: {error}"),
                })?;
            child.kill().map_err(|error| CefariIpcError::Unsupported {
                command: "worker.terminate".to_owned(),
                reason: error.to_string(),
            })?;
            process.status = WorkerStatus::Exited;
            emit_worker_event(
                self.events.as_ref(),
                CefariIpcEvent::Worker(WorkerEvent::Exited(WorkerExitEvent {
                    id: process.id.clone(),
                    worker: process.worker.clone(),
                    code: None,
                    reason: Some("terminated".to_owned()),
                })),
            );
        }
        Ok(WorkerResult::Terminated(cefari_core::WorkerIdResult {
            id: id.to_owned(),
        }))
    }

    fn list(&mut self) -> WorkerListResult {
        self.refresh_statuses();
        WorkerListResult {
            workers: self
                .processes
                .values()
                .map(|process| WorkerState {
                    id: process.id.clone(),
                    worker: process.worker.clone(),
                    status: process.status.clone(),
                })
                .collect(),
        }
    }

    fn refresh_statuses(&mut self) {
        for process in self.processes.values_mut() {
            if matches!(process.status, WorkerStatus::Exited) {
                continue;
            }
            let Ok(mut child) = process.child.lock() else {
                process.status = WorkerStatus::Exited;
                continue;
            };
            match child.has_exited() {
                Ok(true) => process.status = WorkerStatus::Exited,
                Ok(false) => {}
                Err(error) => {
                    debug!(%error, id = %process.id, "failed to refresh worker process status");
                    process.status = WorkerStatus::Exited;
                }
            }
        }
    }

    fn next_worker_id(&mut self, worker: &str) -> String {
        self.next_id += 1;
        format!("{worker}-{}", self.next_id)
    }

    fn process_spec(
        &self,
        id: &str,
        worker: &str,
        entry: &WorkerEntryConfig,
        input_json: &str,
    ) -> Result<WorkerProcessSpec> {
        let entry_path = safe_resource_path(&self.paths.resource_dir, &entry.entry)
            .context("worker entry must resolve inside the resource directory")?;
        let mut args = vec![
            "run".to_owned(),
            "--no-prompt".to_owned(),
            format!("--allow-read={}", entry_path.display()),
        ];
        args.extend(permission_args(&self.paths, &entry.permissions)?);
        args.push(entry_path.display().to_string());
        Ok(WorkerProcessSpec {
            program: "deno".to_owned(),
            args,
            cwd: self.paths.resource_dir.clone(),
            input: serde_json::json!({
                "type": "start",
                "id": id,
                "input": serde_json::from_str::<serde_json::Value>(input_json)
                    .context("worker inputJson must be valid JSON")?,
            })
            .to_string(),
            id: id.to_owned(),
            worker: worker.to_owned(),
        })
    }
}

fn permission_args(
    paths: &RuntimePaths,
    permissions: &cefari_core::WorkerPermissionsConfig,
) -> Result<Vec<String>> {
    let mut args = Vec::new();
    push_path_permission(&mut args, "read", paths, &permissions.read)?;
    push_path_permission(&mut args, "write", paths, &permissions.write)?;
    push_name_permission(&mut args, "net", &permissions.net)?;
    push_name_permission(&mut args, "env", &permissions.env)?;
    push_path_permission(&mut args, "run", paths, &permissions.run)?;
    Ok(args)
}

fn push_path_permission(
    args: &mut Vec<String>,
    name: &str,
    paths: &RuntimePaths,
    permission: &WorkerPermissionConfig,
) -> Result<()> {
    match permission {
        WorkerPermissionConfig::None(value) if value == "none" => {}
        WorkerPermissionConfig::None(value) => {
            anyhow::bail!("--allow-{name} value {value:?} is not supported")
        }
        WorkerPermissionConfig::Allow(values) => {
            let paths = values
                .iter()
                .map(|value| resolve_permission_path(paths, value))
                .collect::<Result<Vec<_>>>()?;
            if !paths.is_empty() {
                args.push(format!(
                    "--allow-{name}={}",
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ));
            }
        }
    }
    Ok(())
}

fn push_name_permission(
    args: &mut Vec<String>,
    name: &str,
    permission: &WorkerPermissionConfig,
) -> Result<()> {
    match permission {
        WorkerPermissionConfig::None(value) if value == "none" => {}
        WorkerPermissionConfig::None(value) => {
            anyhow::bail!("--allow-{name} value {value:?} is not supported")
        }
        WorkerPermissionConfig::Allow(values) if values.is_empty() => {}
        WorkerPermissionConfig::Allow(values) => {
            args.push(format!("--allow-{name}={}", values.join(",")));
        }
    }
    Ok(())
}

fn resolve_permission_path(paths: &RuntimePaths, value: &str) -> Result<PathBuf> {
    if let Some(path) = value.strip_prefix("$appData") {
        return token_path(&paths.data_dir, path);
    }
    if let Some(path) = value.strip_prefix("$cache") {
        return token_path(&paths.cache_dir, path);
    }
    if let Some(path) = value.strip_prefix("$resource") {
        return token_path(&paths.resource_dir, path);
    }
    safe_resource_path(&paths.resource_dir, value)
}

fn token_path(root: &Path, suffix: &str) -> Result<PathBuf> {
    let suffix = suffix.strip_prefix('/').unwrap_or(suffix);
    if suffix.split(['/', '\\']).any(|component| component == "..") {
        anyhow::bail!("permission path must not contain parent traversal");
    }
    Ok(root.join(suffix))
}

fn safe_resource_path(root: &Path, value: &str) -> Result<PathBuf> {
    if Path::new(value).is_absolute() || value.split(['/', '\\']).any(|component| component == "..")
    {
        anyhow::bail!("path must be relative and stay inside the resource directory");
    }
    Ok(root.join(value))
}

fn spawn_stdout_reader(
    events: Arc<dyn WorkerEventSink>,
    id: String,
    worker: String,
    stdout: Box<dyn BufRead + Send>,
) {
    thread::spawn(move || {
        for line in stdout.lines() {
            match line {
                Ok(line) if line.trim().is_empty() => {}
                Ok(line) => handle_worker_stdout_line(events.as_ref(), &id, &worker, &line),
                Err(error) => {
                    error!(%error, %id, %worker, "failed to read worker stdout");
                    break;
                }
            }
        }
    });
}

fn handle_worker_stdout_line(events: &dyn WorkerEventSink, id: &str, worker: &str, line: &str) {
    match serde_json::from_str::<WorkerStdoutEnvelope>(line) {
        Ok(WorkerStdoutEnvelope::Message { payload }) => emit_worker_event(
            events,
            CefariIpcEvent::Worker(WorkerEvent::Message(WorkerMessageEvent {
                id: id.to_owned(),
                worker: worker.to_owned(),
                message_json: payload.to_string(),
            })),
        ),
        Ok(WorkerStdoutEnvelope::Result { payload }) => emit_worker_event(
            events,
            CefariIpcEvent::Worker(WorkerEvent::Message(WorkerMessageEvent {
                id: id.to_owned(),
                worker: worker.to_owned(),
                message_json: payload.to_string(),
            })),
        ),
        Ok(WorkerStdoutEnvelope::Error { error }) => emit_worker_event(
            events,
            CefariIpcEvent::Worker(WorkerEvent::Error(WorkerErrorEvent {
                id: id.to_owned(),
                worker: worker.to_owned(),
                message: error.message,
            })),
        ),
        Err(error) => emit_worker_event(
            events,
            CefariIpcEvent::Worker(WorkerEvent::Error(WorkerErrorEvent {
                id: id.to_owned(),
                worker: worker.to_owned(),
                message: format!("worker stdout protocol error: {error}"),
            })),
        ),
    }
}

fn emit_worker_event(events: &dyn WorkerEventSink, event: CefariIpcEvent) {
    if let Err(error) = events.send_worker_event(event) {
        debug!(%error, "failed to emit worker event");
    }
}

#[derive(Debug, Default)]
struct DenoWorkerProcessSpawner;

impl WorkerProcessSpawner for DenoWorkerProcessSpawner {
    fn spawn(&self, spec: WorkerProcessSpec) -> Result<Box<dyn WorkerChild>> {
        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("failed to spawn {}", spec.program))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(spec.input.as_bytes())
                .context("failed to write worker input")?;
        }
        Ok(Box::new(StdWorkerChild { child }))
    }
}

struct StdWorkerChild {
    child: Child,
}

impl WorkerChild for StdWorkerChild {
    fn take_stdout(&mut self) -> Option<Box<dyn BufRead + Send>> {
        self.child
            .stdout
            .take()
            .map(|stdout| Box::new(BufReader::new(stdout)) as Box<dyn BufRead + Send>)
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    fn has_exited(&mut self) -> std::io::Result<bool> {
        self.child.try_wait().map(|status| status.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cefari_core::{WorkerEntryConfig, WorkerPermissionsConfig};
    use std::{
        io::Cursor,
        sync::{Arc, Mutex},
    };

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<CefariIpcEvent>>,
    }

    impl WorkerEventSink for RecordingSink {
        fn send_worker_event(&self, event: CefariIpcEvent) -> Result<()> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingSpawner {
        specs: Mutex<Vec<WorkerProcessSpec>>,
    }

    impl WorkerProcessSpawner for RecordingSpawner {
        fn spawn(&self, spec: WorkerProcessSpec) -> Result<Box<dyn WorkerChild>> {
            self.specs.lock().unwrap().push(spec);
            Ok(Box::new(FakeWorkerChild {
                stdout: Some(Box::new(Cursor::new(
                    br#"{"type":"message","payload":{"progress":0.5}}
"#
                    .to_vec(),
                ))),
                killed: false,
                exited: false,
            }))
        }
    }

    struct FakeWorkerChild {
        stdout: Option<Box<dyn BufRead + Send>>,
        killed: bool,
        exited: bool,
    }

    impl WorkerChild for FakeWorkerChild {
        fn take_stdout(&mut self) -> Option<Box<dyn BufRead + Send>> {
            self.stdout.take()
        }

        fn kill(&mut self) -> std::io::Result<()> {
            self.killed = true;
            self.exited = true;
            Ok(())
        }

        fn has_exited(&mut self) -> std::io::Result<bool> {
            Ok(self.exited)
        }
    }

    #[test]
    fn builds_deno_command_for_configured_worker() {
        let sink = Arc::new(RecordingSink::default());
        let spawner = Arc::new(RecordingSpawner::default());
        let mut manager =
            DesktopWorkerManager::with_spawner(worker_config(), paths(), sink, spawner.clone());

        let result = manager
            .dispatch(&WorkerCommand::Spawn(WorkerSpawnRequest {
                worker: "thumbnailer".to_owned(),
                input_json: r#"{"imageId":"abc"}"#.to_owned(),
            }))
            .unwrap();

        assert!(matches!(result, WorkerResult::Spawned(_)));
        let specs = spawner.specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].program, "deno");
        assert!(specs[0].args.contains(&"run".to_owned()));
        assert!(specs[0].args.contains(&"--no-prompt".to_owned()));
        assert!(specs[0].args.contains(&format!(
            "--allow-write={}",
            paths().data_dir.join("cache").display()
        )));
        assert_eq!(
            specs[0].input,
            r#"{"id":"thumbnailer-1","input":{"imageId":"abc"},"type":"start"}"#
        );
    }

    #[test]
    fn rejects_unknown_worker_without_spawning() {
        let spawner = Arc::new(RecordingSpawner::default());
        let mut manager = DesktopWorkerManager::with_spawner(
            worker_config(),
            paths(),
            Arc::new(RecordingSink::default()),
            spawner.clone(),
        );

        let error = manager
            .dispatch(&WorkerCommand::Spawn(WorkerSpawnRequest {
                worker: "missing".to_owned(),
                input_json: "{}".to_owned(),
            }))
            .unwrap_err();

        assert!(matches!(error, CefariIpcError::InvalidCommand { .. }));
        assert!(spawner.specs.lock().unwrap().is_empty());
    }

    #[test]
    fn list_and_terminate_track_worker_state() {
        let sink = Arc::new(RecordingSink::default());
        let mut manager = DesktopWorkerManager::with_spawner(
            worker_config(),
            paths(),
            sink.clone(),
            Arc::new(RecordingSpawner::default()),
        );
        let spawned = manager
            .dispatch(&WorkerCommand::Spawn(WorkerSpawnRequest {
                worker: "thumbnailer".to_owned(),
                input_json: "{}".to_owned(),
            }))
            .unwrap();
        let id = match spawned {
            WorkerResult::Spawned(result) => result.id,
            _ => panic!("expected spawned"),
        };

        let listed = manager.dispatch(&WorkerCommand::List).unwrap();
        assert_eq!(
            listed,
            WorkerResult::List(WorkerListResult {
                workers: vec![WorkerState {
                    id: id.clone(),
                    worker: "thumbnailer".to_owned(),
                    status: WorkerStatus::Running,
                }],
            })
        );

        assert_eq!(
            manager
                .dispatch(&WorkerCommand::Terminate(cefari_core::WorkerIdRequest {
                    id: id.clone()
                }))
                .unwrap(),
            WorkerResult::Terminated(cefari_core::WorkerIdResult { id })
        );
        assert!(matches!(
            sink.events.lock().unwrap().last(),
            Some(CefariIpcEvent::Worker(WorkerEvent::Exited(event)))
                if event.reason.as_deref() == Some("terminated")
        ));
    }

    #[test]
    fn list_updates_completed_worker_state() {
        let mut manager = DesktopWorkerManager::with_spawner(
            worker_config(),
            paths(),
            Arc::new(RecordingSink::default()),
            Arc::new(ExitedSpawner),
        );
        let spawned = manager
            .dispatch(&WorkerCommand::Spawn(WorkerSpawnRequest {
                worker: "thumbnailer".to_owned(),
                input_json: "{}".to_owned(),
            }))
            .unwrap();
        let id = match spawned {
            WorkerResult::Spawned(result) => result.id,
            _ => panic!("expected spawned"),
        };

        assert_eq!(
            manager.dispatch(&WorkerCommand::List).unwrap(),
            WorkerResult::List(WorkerListResult {
                workers: vec![WorkerState {
                    id,
                    worker: "thumbnailer".to_owned(),
                    status: WorkerStatus::Exited,
                }],
            })
        );
    }

    #[test]
    fn worker_stdout_message_emits_ipc_event() {
        let sink = RecordingSink::default();

        handle_worker_stdout_line(
            &sink,
            "worker-1",
            "thumbnailer",
            r#"{"type":"message","payload":{"progress":0.5}}"#,
        );

        assert!(matches!(
            sink.events.lock().unwrap().as_slice(),
            [CefariIpcEvent::Worker(WorkerEvent::Message(event))]
                if event.id == "worker-1"
                    && event.worker == "thumbnailer"
                    && event.message_json == r#"{"progress":0.5}"#
        ));
    }

    fn worker_config() -> WorkerConfig {
        WorkerConfig {
            entries: BTreeMap::from([(
                "thumbnailer".to_owned(),
                WorkerEntryConfig {
                    entry: "workers/thumbnailer.ts".to_owned(),
                    permissions: WorkerPermissionsConfig {
                        read: WorkerPermissionConfig::Allow(vec!["$appData/uploads".to_owned()]),
                        write: WorkerPermissionConfig::Allow(vec!["$appData/cache".to_owned()]),
                        net: WorkerPermissionConfig::None("none".to_owned()),
                        env: WorkerPermissionConfig::None("none".to_owned()),
                        run: WorkerPermissionConfig::None("none".to_owned()),
                    },
                },
            )]),
        }
    }

    struct ExitedSpawner;

    impl WorkerProcessSpawner for ExitedSpawner {
        fn spawn(&self, _spec: WorkerProcessSpec) -> Result<Box<dyn WorkerChild>> {
            Ok(Box::new(FakeWorkerChild {
                stdout: None,
                killed: false,
                exited: true,
            }))
        }
    }

    fn paths() -> RuntimePaths {
        RuntimePaths {
            config_dir: PathBuf::from("/tmp/cefari/config"),
            config_file: PathBuf::from("/tmp/cefari/config/cefari.json"),
            data_dir: PathBuf::from("/tmp/cefari/data"),
            cache_dir: PathBuf::from("/tmp/cefari/cache"),
            log_dir: PathBuf::from("/tmp/cefari/logs"),
            resource_dir: PathBuf::from("/tmp/cefari/resources"),
            update_dir: PathBuf::from("/tmp/cefari/updates"),
        }
    }
}
