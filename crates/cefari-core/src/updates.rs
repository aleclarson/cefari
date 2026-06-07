use cargo_packager_updater::{Config, semver::Version};

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
}
