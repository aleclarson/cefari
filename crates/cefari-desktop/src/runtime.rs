use std::path::{Path, PathBuf};

use anyhow::Result;
use cefari_core::{
    CefariConfig, CefariServiceSpec, RuntimePaths, UpdateCheckConfig, UpdateCheckState,
    check_for_update, install_service, install_update, load_config, service_manager,
    service_status, start_service, stop_service,
};

const DAEMON_EXECUTABLE_NAME: &str = if cfg!(windows) {
    "cefari-daemon.exe"
} else {
    "cefari-daemon"
};

pub struct RuntimeOperations {
    config: CefariConfig,
    paths: RuntimePaths,
}

impl RuntimeOperations {
    pub fn load(paths: &RuntimePaths) -> Result<Self> {
        let config = load_desktop_config(&paths.config_file)?;
        Ok(Self {
            config,
            paths: paths.clone(),
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
            current_version: env!("CARGO_PKG_VERSION").to_owned(),
            endpoints,
            public_key: self.config.updates.public_key.clone().unwrap_or_default(),
        }
    }

    #[allow(dead_code)]
    pub fn update_state(&self) -> Result<UpdateCheckState> {
        let (state, _) = check_for_update(&self.update_check_config())?;
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

    pub fn daemon_service_spec(&self) -> CefariServiceSpec {
        CefariServiceSpec::daemon(self.daemon_program())
            .with_arg("--foreground")
            .with_working_directory(&self.paths.data_dir)
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

fn load_desktop_config(path: &Path) -> Result<CefariConfig> {
    if path.exists() {
        load_config(path).map_err(Into::into)
    } else {
        Ok(CefariConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use cefari_core::{AppIdentity, RuntimePaths, UpdateCheckState};

    use super::{DAEMON_EXECUTABLE_NAME, RuntimeOperations};

    #[test]
    fn defaults_to_unconfigured_updates_without_config_file() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");
        let runtime = RuntimeOperations::load(&paths).expect("default config should load");

        let update_config = runtime.update_check_config();

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
    }
}
