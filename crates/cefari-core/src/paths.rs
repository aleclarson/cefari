use std::path::PathBuf;

use directories::ProjectDirs;

use crate::{Error, Result};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppIdentity {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

impl AppIdentity {
    #[must_use]
    pub fn cefari() -> Self {
        Self {
            qualifier: "dev".to_owned(),
            organization: "Cefari".to_owned(),
            application: "Cefari".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimePaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub resource_dir: PathBuf,
    pub update_dir: PathBuf,
}

impl RuntimePaths {
    pub fn resolve(identity: &AppIdentity) -> Result<Self> {
        let dirs = ProjectDirs::from(
            &identity.qualifier,
            &identity.organization,
            &identity.application,
        )
        .ok_or_else(|| Error::ProjectDirectoriesUnavailable {
            qualifier: identity.qualifier.clone(),
            organization: identity.organization.clone(),
            application: identity.application.clone(),
        })?;

        let config_dir = dirs.config_dir().to_owned();
        let data_dir = dirs.data_dir().to_owned();
        let cache_dir = dirs.cache_dir().to_owned();

        Ok(Self {
            config_file: config_dir.join("cefari.json"),
            log_dir: data_dir.join("logs"),
            resource_dir: data_dir.join("resources"),
            update_dir: data_dir.join("updates"),
            config_dir,
            data_dir,
            cache_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AppIdentity, RuntimePaths};

    #[test]
    fn resolves_default_paths() {
        let paths = RuntimePaths::resolve(&AppIdentity::cefari()).expect("paths should resolve");

        assert!(paths.config_file.ends_with("cefari.json"));
        assert!(paths.log_dir.ends_with("logs"));
        assert!(paths.resource_dir.ends_with("resources"));
        assert!(paths.update_dir.ends_with("updates"));
    }
}
