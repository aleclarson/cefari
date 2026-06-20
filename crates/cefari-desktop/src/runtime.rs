use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

use anyhow::Result;
use cefari_core::{
    AppConfig, BrowserConfig, CEFARI_DAEMON_LOG_ENV, CefariConfig, CefariServiceSpec, DaemonConfig,
    NativeResourceConfig, PendingUpdate, RuntimeLogConfig, RuntimePaths, UpdateCheckConfig, UpdateCheckState,
    WorkerConfig, check_for_update, install_service, install_update, load_config,
    packaged_resources_dir, resolve_resource, service_manager, service_status, start_service,
    stop_service, update_id,
};

use crate::desktop_daemon::DaemonProcessConfig;

const CEFARI_DAEMON_DEV_ENTRY_ENV: &str = "CEFARI_DAEMON_DEV_ENTRY";
const CEFARI_DAEMON_DEV_CWD_ENV: &str = "CEFARI_DAEMON_DEV_CWD";
const CEFARI_DAEMON_RESOURCES_ENV: &str = "CEFARI_DAEMON_RESOURCES";
const CEFARI_CONFIG_FILE_ENV: &str = "CEFARI_CONFIG_FILE";

pub struct RuntimeOperations {
    config: CefariConfig,
    paths: RuntimePaths,
    updates: Mutex<RuntimeUpdateState>,
}

#[derive(Debug, Default)]
struct RuntimeUpdateState {
    state: Option<UpdateCheckState>,
    pending: Option<PendingUpdate>,
    ready_to_restart_version: Option<String>,
}

impl RuntimeOperations {
    pub fn load(paths: &RuntimePaths) -> Result<Self> {
        let config = load_desktop_config(
            &paths.config_file,
            std::env::var_os(CEFARI_CONFIG_FILE_ENV).map(PathBuf::from),
        )?;
        Ok(Self {
            config,
            paths: paths.clone(),
            updates: Mutex::default(),
        })
    }

    pub fn update_check_config(&self) -> UpdateCheckConfig {
        let endpoints = self
            .config
            .updates
            .endpoint
            .iter()
            .filter(|endpoint| !endpoint.is_empty())
            .cloned()
            .collect();

        UpdateCheckConfig {
            current_version: self.config.app.version.clone(),
            endpoints,
            public_key: self.config.updates.public_key.clone().unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    pub fn update_state(&self) -> Result<UpdateCheckState> {
        let updates = self
            .updates
            .lock()
            .map_err(|error| anyhow::anyhow!("update state lock poisoned: {error}"))?;
        if updates.ready_to_restart_version.is_some() {
            return Ok(UpdateCheckState::ReadyToRestart);
        }
        if let Some(state) = &updates.state {
            return Ok(state.clone());
        }
        drop(updates);

        let (state, _) = check_for_update(&self.update_check_config())?;
        Ok(state)
    }

    #[allow(dead_code)]
    pub fn update_check(&self) -> Result<UpdateCheckState> {
        let (state, update) = check_for_update(&self.update_check_config())?;
        let mut updates = self
            .updates
            .lock()
            .map_err(|error| anyhow::anyhow!("update state lock poisoned: {error}"))?;
        updates.state = Some(state.clone());
        updates.pending = update;
        updates.ready_to_restart_version = None;
        Ok(state)
    }

    #[allow(dead_code)]
    pub fn check_and_install_update(&self) -> Result<UpdateCheckState> {
        let (state, update) = check_for_update(&self.update_check_config())?;
        if let Some(update) = update {
            install_update(&update)?;
        }
        Ok(state)
    }

    pub fn apply_update(&self, requested_update_id: Option<&str>) -> Result<AppliedUpdate> {
        let update = {
            let mut updates = self
                .updates
                .lock()
                .map_err(|error| anyhow::anyhow!("update state lock poisoned: {error}"))?;
            let update = updates
                .pending
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no checked update is available to apply"))?;
            let available_update_id = update_id(update);
            if requested_update_id.is_some_and(|requested| requested != available_update_id) {
                anyhow::bail!("checked update id does not match the requested update id");
            }
            let update = update.clone();
            updates.state = Some(UpdateCheckState::Applying);
            update
        };

        if let Err(error) = install_update(&update) {
            let mut updates = self
                .updates
                .lock()
                .map_err(|error| anyhow::anyhow!("update state lock poisoned: {error}"))?;
            updates.state = Some(UpdateCheckState::Failed {
                message: error.to_string(),
            });
            return Err(error.into());
        }

        let applied = AppliedUpdate {
            version: update.version.clone(),
        };
        let mut updates = self
            .updates
            .lock()
            .map_err(|error| anyhow::anyhow!("update state lock poisoned: {error}"))?;
        updates.state = Some(UpdateCheckState::ReadyToRestart);
        updates.pending = None;
        updates.ready_to_restart_version = Some(applied.version.clone());
        Ok(applied)
    }

    pub fn app_config(&self) -> &AppConfig {
        &self.config.app
    }

    pub fn browser_config(&self) -> &BrowserConfig {
        &self.config.browser
    }

    pub fn deep_link_schemes(&self) -> &[String] {
        &self.config.deep_links.schemes
    }

    pub fn worker_config(&self) -> &WorkerConfig {
        &self.config.workers
    }

    pub fn daemon_configured(&self) -> bool {
        self.config.daemon.enabled || std::env::var_os(CEFARI_DAEMON_DEV_ENTRY_ENV).is_some()
    }

    pub fn daemon_service_spec(&self) -> Result<CefariServiceSpec> {
        let daemon = self.daemon_process_config()?;
        let mut spec = CefariServiceSpec::daemon(daemon.program)
            .with_arg("--foreground")
            .with_working_directory(daemon.working_directory);
        for (key, value) in daemon.environment {
            spec = spec.with_environment(
                key.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            );
        }
        Ok(spec)
    }

    pub fn daemon_process_config(&self) -> Result<DaemonProcessConfig> {
        if let Some(entry) = std::env::var_os(CEFARI_DAEMON_DEV_ENTRY_ENV) {
            let working_directory = std::env::var_os(CEFARI_DAEMON_DEV_CWD_ENV)
                .map(PathBuf::from)
                .unwrap_or_else(|| self.paths.resource_dir.clone());
            let mut environment = vec![(
                CEFARI_DAEMON_LOG_ENV.into(),
                daemon_log_path(&self.paths).into(),
            )];
            if let Some(resources) = std::env::var_os(CEFARI_DAEMON_RESOURCES_ENV) {
                environment.push((CEFARI_DAEMON_RESOURCES_ENV.into(), resources));
            }
            return Ok(DaemonProcessConfig {
                program: PathBuf::from("deno"),
                args: vec![OsString::from("run"), OsString::from("-A"), entry],
                working_directory,
                environment,
            });
        }

        Ok(DaemonProcessConfig {
            program: self.daemon_program()?,
            args: Vec::new(),
            working_directory: self.paths.data_dir.clone(),
            environment: daemon_environment(&self.paths, &self.config.daemon)?,
        })
    }

    #[allow(dead_code)]
    pub fn install_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        install_service(manager.as_ref(), &self.daemon_service_spec()?)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn start_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        start_service(manager.as_ref(), &self.daemon_service_spec()?)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        stop_service(manager.as_ref(), &self.daemon_service_spec()?)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn daemon_service_status(&self) -> Result<String> {
        let manager = service_manager(None)?;
        let status = service_status(manager.as_ref(), &self.daemon_service_spec()?)?;
        Ok(format!("{status:?}"))
    }

    fn daemon_program(&self) -> Result<PathBuf> {
        daemon_executable_path(&self.config.daemon, &self.paths.resource_dir)
    }
}

fn daemon_executable_path(config: &DaemonConfig, resource_dir: &Path) -> Result<PathBuf> {
    if !config.enabled {
        anyhow::bail!("daemon is not configured");
    }
    let executable = config
        .executable
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("daemon executable is not configured"))?;
    let executable_path = Path::new(executable);
    if executable_path.is_absolute()
        || executable_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("daemon executable must be a relative path inside resources");
    }
    Ok(resource_dir.join(executable_path))
}

fn daemon_log_path(paths: &RuntimePaths) -> String {
    RuntimeLogConfig::new(paths)
        .daemon
        .file_path()
        .display()
        .to_string()
}

fn daemon_environment(paths: &RuntimePaths, daemon: &DaemonConfig) -> Result<Vec<(OsString, OsString)>> {
    let mut environment = vec![(
        OsString::from(CEFARI_DAEMON_LOG_ENV),
        OsString::from(daemon_log_path(paths)),
    )];
    environment.push((
        OsString::from(CEFARI_DAEMON_RESOURCES_ENV),
        OsString::from(daemon_resources_json(paths, &daemon.native)?),
    ));
    Ok(environment)
}

fn daemon_resources_json(paths: &RuntimePaths, native: &[NativeResourceConfig]) -> Result<String> {
    let native_paths = native
        .iter()
        .map(|resource| {
            let path = daemon_native_resource_path(paths, resource)?;
            Ok((resource.id.clone(), path.display().to_string()))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    serde_json::to_string(&serde_json::json!({
        "resourceDir": paths.resource_dir.display().to_string(),
        "nativeDir": paths.resource_dir.join("daemon").join("native").display().to_string(),
        "native": native_paths,
    }))
    .map_err(Into::into)
}

fn daemon_native_resource_path(paths: &RuntimePaths, resource: &NativeResourceConfig) -> Result<PathBuf> {
    let path = Path::new(&resource.path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("daemon native resource path must be a relative path inside resources");
    }
    Ok(paths.resource_dir.join(path))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppliedUpdate {
    pub version: String,
}

fn load_desktop_config(path: &Path, override_path: Option<PathBuf>) -> Result<CefariConfig> {
    if let Some(override_path) = override_path {
        return load_config(override_path).map_err(Into::into);
    }

    if path.exists() {
        return load_config(path).map_err(Into::into);
    }

    if let Some(packaged_config) = packaged_config_file() {
        return load_config(packaged_config).map_err(Into::into);
    }

    Ok(CefariConfig::default())
}

fn packaged_config_file() -> Option<PathBuf> {
    platform_package_formats()
        .iter()
        .filter_map(|format| packaged_resources_dir(*format).ok())
        .find_map(|dir| resolve_resource(dir, "config/cefari.json").ok())
}

#[cfg(target_os = "macos")]
fn platform_package_formats() -> &'static [cefari_core::PackageFormat] {
    &[
        cefari_core::PackageFormat::App,
        cefari_core::PackageFormat::Dmg,
    ]
}

#[cfg(target_os = "windows")]
fn platform_package_formats() -> &'static [cefari_core::PackageFormat] {
    &[
        cefari_core::PackageFormat::Nsis,
        cefari_core::PackageFormat::Wix,
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_package_formats() -> &'static [cefari_core::PackageFormat] {
    &[
        cefari_core::PackageFormat::Deb,
        cefari_core::PackageFormat::AppImage,
        cefari_core::PackageFormat::Pacman,
    ]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
fn platform_package_formats() -> &'static [cefari_core::PackageFormat] {
    &[]
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, path::PathBuf, sync::Mutex};

    use cefari_core::{
        AppIdentity, CEFARI_DAEMON_LOG_ENV, CefariConfig, DaemonConfig, RuntimePaths,
        UpdateCheckState,
    };

    use super::{
        CEFARI_DAEMON_RESOURCES_ENV, RuntimeOperations, RuntimeUpdateState,
        daemon_executable_path, load_desktop_config,
    };

    #[test]
    fn defaults_to_unconfigured_updates_without_config_file() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let runtime = RuntimeOperations::load(&paths).expect("default config should load");

        let update_config = runtime.update_check_config();

        assert_eq!(update_config.current_version, "0.0.0");
        assert!(!update_config.is_configured());
        assert_eq!(
            runtime.update_state().expect("unconfigured update check"),
            UpdateCheckState::NotConfigured
        );
    }

    #[test]
    fn loads_explicit_desktop_config_override() {
        let root = temp_dir("config-override");
        fs::create_dir_all(&root).expect("temp dir should exist");
        let config_file = root.join("cefari.json");
        fs::write(
            &config_file,
            r#"{
  "app": {
    "identifier": "dev.cefari.override",
    "display_name": "Override App",
    "version": "1.2.3"
  },
  "workers": {
    "entries": {
      "thumbnailer": {
        "target": {
          "kind": "denoSource",
          "entry": "workers/thumbnailer.ts",
          "permissions": {
            "read": ["$appData/uploads"]
          }
        }
      }
    }
  }
}"#,
        )
        .expect("config should be written");

        let config = load_desktop_config(&root.join("missing.json"), Some(config_file))
            .expect("override config should load");

        assert_eq!(config.app.identifier, "dev.cefari.override");
        assert!(config.workers.entries.contains_key("thumbnailer"));
    }

    #[test]
    fn default_config_has_no_daemon_service_spec() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let runtime = RuntimeOperations::load(&paths).expect("default config should load");

        let error = runtime
            .daemon_service_spec()
            .expect_err("default runtime should not configure daemon");

        assert!(error.to_string().contains("daemon is not configured"));
    }

    #[test]
    fn builds_daemon_service_spec_from_configured_runtime_path() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let runtime = runtime_with_daemon(
            &paths,
            DaemonConfig {
                enabled: true,
                executable: Some("daemon/example-daemon".to_owned()),
                native: Vec::new(),
            },
        );
        let spec = runtime
            .daemon_service_spec()
            .expect("daemon should be configured");

        assert_eq!(spec.label.to_qualified_name(), "dev.cefari.daemon");
        assert_eq!(
            spec.program,
            paths.resource_dir.join("daemon/example-daemon")
        );
        assert_eq!(spec.working_directory, Some(paths.data_dir));
        assert_eq!(
            spec.environment,
            vec![
                (
                    CEFARI_DAEMON_LOG_ENV.to_owned(),
                    paths.log_dir.join("daemon.log").display().to_string()
                ),
                (
                    CEFARI_DAEMON_RESOURCES_ENV.to_owned(),
                    serde_json::json!({
                        "native": {},
                        "nativeDir": paths.resource_dir.join("daemon").join("native").display().to_string(),
                        "resourceDir": paths.resource_dir.display().to_string(),
                    })
                    .to_string(),
                ),
            ]
        );
        assert_eq!(
            runtime
                .daemon_process_config()
                .expect("daemon process config")
                .args,
            Vec::<OsString>::new()
        );
    }

    #[test]
    fn rejects_daemon_executables_outside_resources() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        for executable in ["/tmp/daemon", "../daemon"] {
            let error = daemon_executable_path(
                &DaemonConfig {
                    enabled: true,
                    executable: Some(executable.to_owned()),
                    native: Vec::new(),
                },
                &paths.resource_dir,
            )
            .expect_err("daemon path should be rejected");

            assert!(
                error
                    .to_string()
                    .contains("daemon executable must be a relative path")
            );
        }
    }

    fn runtime_with_daemon(paths: &RuntimePaths, daemon: DaemonConfig) -> RuntimeOperations {
        RuntimeOperations {
            config: CefariConfig {
                daemon,
                ..CefariConfig::default()
            },
            paths: paths.clone(),
            updates: Mutex::<RuntimeUpdateState>::default(),
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-runtime-{label}-{suffix}"))
    }
}
