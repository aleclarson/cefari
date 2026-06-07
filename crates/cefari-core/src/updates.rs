use cargo_packager_updater::{Config, Update, check_update, semver::Version};

use crate::{Error, Result};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UpdateCheckConfig {
    pub current_version: String,
    pub endpoints: Vec<String>,
    pub public_key: String,
}

impl UpdateCheckConfig {
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.endpoints.is_empty() && !self.public_key.is_empty()
    }

    pub fn prepare(&self) -> Result<PreparedUpdateCheck> {
        let current_version = self.current_version.parse::<Version>().map_err(|source| {
            Error::InvalidCurrentVersion {
                version: self.current_version.clone(),
                source,
            }
        })?;

        let endpoints = self
            .endpoints
            .iter()
            .map(|endpoint| {
                endpoint
                    .parse()
                    .map_err(|source| Error::InvalidUpdateEndpoint {
                        endpoint: endpoint.clone(),
                        source,
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(PreparedUpdateCheck {
            current_version,
            config: Config {
                endpoints,
                pubkey: self.public_key.clone(),
                ..Config::default()
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct PreparedUpdateCheck {
    pub current_version: Version,
    pub config: Config,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UpdateCheckState {
    NotConfigured,
    Ready,
    Checking,
    NoUpdate,
    UpdateAvailable { version: String },
    Failed { message: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AvailableUpdate {
    pub version: String,
    pub current_version: String,
    pub target: String,
    pub download_url: String,
    pub notes: Option<String>,
}

impl From<&Update> for AvailableUpdate {
    fn from(update: &Update) -> Self {
        Self {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            target: update.target.clone(),
            download_url: update.download_url.to_string(),
            notes: update.body.clone(),
        }
    }
}

pub fn check_for_update(config: &UpdateCheckConfig) -> Result<(UpdateCheckState, Option<Update>)> {
    if !config.is_configured() {
        return Ok((UpdateCheckState::NotConfigured, None));
    }

    let prepared = config.prepare()?;
    match check_update(prepared.current_version, prepared.config) {
        Ok(Some(update)) => {
            let available = AvailableUpdate::from(&update);
            Ok((
                UpdateCheckState::UpdateAvailable {
                    version: available.version,
                },
                Some(update),
            ))
        }
        Ok(None) => Ok((UpdateCheckState::NoUpdate, None)),
        Err(source) => Err(Error::Updater {
            operation: "check",
            source,
        }),
    }
}

pub fn install_update(update: &Update) -> Result<()> {
    update
        .download_and_install()
        .map_err(|source| Error::Updater {
            operation: "download-and-install",
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::{UpdateCheckConfig, UpdateCheckState};
    use crate::Error;

    #[test]
    fn detects_unconfigured_updates() {
        let config = UpdateCheckConfig {
            current_version: "0.1.0".to_owned(),
            endpoints: Vec::new(),
            public_key: String::new(),
        };

        assert!(!config.is_configured());
    }

    #[test]
    fn prepares_updater_config() {
        let config = UpdateCheckConfig {
            current_version: "0.1.0".to_owned(),
            endpoints: vec!["https://example.com/{{target}}/{{arch}}".to_owned()],
            public_key: "public-key".to_owned(),
        };

        let prepared = config.prepare().expect("update config should prepare");

        assert_eq!(prepared.current_version.to_string(), "0.1.0");
        assert_eq!(prepared.config.endpoints.len(), 1);
        assert_eq!(prepared.config.pubkey, "public-key");
    }

    #[test]
    fn reports_invalid_endpoint() {
        let config = UpdateCheckConfig {
            current_version: "0.1.0".to_owned(),
            endpoints: vec!["not a url".to_owned()],
            public_key: "public-key".to_owned(),
        };

        let error = config.prepare().expect_err("invalid endpoint should fail");
        assert!(matches!(error, Error::InvalidUpdateEndpoint { .. }));
    }

    #[test]
    fn reports_invalid_current_version() {
        let config = UpdateCheckConfig {
            current_version: "not-semver".to_owned(),
            endpoints: vec!["https://example.com".to_owned()],
            public_key: "public-key".to_owned(),
        };

        let error = config.prepare().expect_err("invalid version should fail");
        assert!(matches!(error, Error::InvalidCurrentVersion { .. }));
    }

    #[test]
    fn models_update_state() {
        let state = UpdateCheckState::UpdateAvailable {
            version: "0.2.0".to_owned(),
        };

        assert_eq!(
            state,
            UpdateCheckState::UpdateAvailable {
                version: "0.2.0".to_owned()
            }
        );
    }

    #[test]
    fn check_reports_not_configured_without_network() {
        let config = UpdateCheckConfig {
            current_version: "0.1.0".to_owned(),
            endpoints: Vec::new(),
            public_key: String::new(),
        };

        let (state, update) =
            super::check_for_update(&config).expect("unconfigured updates should not fail");

        assert_eq!(state, UpdateCheckState::NotConfigured);
        assert!(update.is_none());
    }
}
