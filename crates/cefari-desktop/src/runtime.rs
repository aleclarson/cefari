use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::Result;
use cefari_core::{
    AppConfig, CEFARI_DAEMON_LOG_ENV, CefariConfig, CefariServiceSpec, PendingUpdate,
    RuntimeLogConfig, RuntimePaths, UpdateCheckConfig, UpdateCheckState, check_for_update,
    install_service, install_update, load_config, service_manager, service_status, start_service,
    stop_service, update_id,
};

const DAEMON_EXECUTABLE_NAME: &str = if cfg!(windows) {
    "cefari-daemon.exe"
} else {
    "cefari-daemon"
};

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
        let config = load_desktop_config(&paths.config_file)?;
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

    pub fn daemon_service_spec(&self) -> CefariServiceSpec {
        let log_config = RuntimeLogConfig::new(&self.paths);
        CefariServiceSpec::daemon(self.daemon_program())
            .with_arg("--foreground")
            .with_working_directory(&self.paths.data_dir)
            .with_environment(
                CEFARI_DAEMON_LOG_ENV,
                log_config.daemon.file_path().display().to_string(),
            )
    }

    #[allow(dead_code)]
    pub fn install_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        install_service(manager.as_ref(), &self.daemon_service_spec())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn start_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        start_service(manager.as_ref(), &self.daemon_service_spec())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stop_daemon_service(&self) -> Result<()> {
        let manager = service_manager(None)?;
        stop_service(manager.as_ref(), &self.daemon_service_spec())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn daemon_service_status(&self) -> Result<String> {
        let manager = service_manager(None)?;
        let status = service_status(manager.as_ref(), &self.daemon_service_spec())?;
        Ok(format!("{status:?}"))
    }

    fn daemon_program(&self) -> PathBuf {
        self.paths
            .resource_dir
            .join("daemon")
            .join(DAEMON_EXECUTABLE_NAME)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppliedUpdate {
    pub version: String,
}

fn load_desktop_config(path: &Path) -> Result<CefariConfig> {
    if path.exists() {
        load_config(path).map_err(Into::into)
    } else {
        Ok(CefariConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use cefari_core::{AppIdentity, CEFARI_DAEMON_LOG_ENV, RuntimePaths, UpdateCheckState};

    use super::{DAEMON_EXECUTABLE_NAME, RuntimeOperations};

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
    fn builds_daemon_service_spec_from_runtime_paths() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let runtime = RuntimeOperations::load(&paths).expect("default config should load");
        let spec = runtime.daemon_service_spec();

        assert_eq!(spec.label.to_qualified_name(), "dev.cefari.daemon");
        assert!(spec.program.ends_with(DAEMON_EXECUTABLE_NAME));
        assert_eq!(spec.working_directory, Some(paths.data_dir));
        assert_eq!(
            spec.environment,
            vec![(
                CEFARI_DAEMON_LOG_ENV.to_owned(),
                paths.log_dir.join("daemon.log").display().to_string()
            )]
        );
    }
}
