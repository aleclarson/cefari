use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CefariConfig {
    pub app: AppConfig,
    pub browser: BrowserConfig,
    pub daemon: DaemonConfig,
    pub deep_links: DeepLinkConfig,
    pub updates: UpdateConfig,
    pub service: ServiceConfig,
    pub workers: WorkerConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub identifier: String,
    pub display_name: String,
    pub version: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            identifier: "dev.cefari.app".to_owned(),
            display_name: "Cefari".to_owned(),
            version: "0.0.0".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserConfig {
    pub webgpu: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct UpdateConfig {
    pub endpoint: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DaemonConfig {
    pub enabled: bool,
    pub executable: Option<String>,
    pub native: Vec<NativeResourceConfig>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DeepLinkConfig {
    pub schemes: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerConfig {
    pub entries: std::collections::BTreeMap<String, WorkerEntryConfig>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerEntryConfig {
    pub target: WorkerTargetConfig,
    pub native: Vec<NativeResourceConfig>,
}

impl Default for WorkerEntryConfig {
    fn default() -> Self {
        Self {
            target: WorkerTargetConfig::default(),
            native: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NativeResourceConfig {
    pub id: String,
    pub target: String,
    pub path: String,
    pub executable: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WorkerTargetConfig {
    #[serde(rename = "denoSource")]
    DenoSource(WorkerDenoSourceConfig),
    #[serde(rename = "executable")]
    Executable(WorkerExecutableConfig),
}

impl Default for WorkerTargetConfig {
    fn default() -> Self {
        Self::DenoSource(WorkerDenoSourceConfig::default())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerDenoSourceConfig {
    pub entry: String,
    pub permissions: WorkerPermissionsConfig,
}

impl Default for WorkerDenoSourceConfig {
    fn default() -> Self {
        Self {
            entry: String::new(),
            permissions: WorkerPermissionsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerExecutableConfig {
    pub program: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerPermissionsConfig {
    pub read: WorkerPermissionConfig,
    pub write: WorkerPermissionConfig,
    pub net: WorkerPermissionConfig,
    pub env: WorkerPermissionConfig,
    pub run: WorkerPermissionConfig,
    pub ffi: WorkerPermissionConfig,
}

impl Default for WorkerPermissionsConfig {
    fn default() -> Self {
        Self {
            read: WorkerPermissionConfig::default(),
            write: WorkerPermissionConfig::default(),
            net: WorkerPermissionConfig::default(),
            env: WorkerPermissionConfig::default(),
            run: WorkerPermissionConfig::default(),
            ffi: WorkerPermissionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkerPermissionConfig {
    None(String),
    Allow(Vec<String>),
}

impl Default for WorkerPermissionConfig {
    fn default() -> Self {
        Self::None("none".to_owned())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServiceConfig {
    pub name: String,
    pub display_name: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: "cefari-daemon".to_owned(),
            display_name: "Cefari Daemon".to_owned(),
        }
    }
}

pub fn load_config(path: impl AsRef<Path>) -> Result<CefariConfig> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| Error::ReadConfig {
        path: path.to_owned(),
        source,
    })?;

    serde_json::from_str(&contents).map_err(|source| Error::ParseConfig {
        path: path.to_owned(),
        source,
    })
}

pub fn save_config(path: impl AsRef<Path>, config: &CefariConfig) -> Result<()> {
    let path = path.as_ref();
    let contents = serde_json::to_string_pretty(config)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::CreateConfigDirectory {
            path: parent.to_owned(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| Error::WriteConfig {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, BrowserConfig, CefariConfig, DaemonConfig, ServiceConfig,
        WorkerPermissionConfig, WorkerTargetConfig,
    };

    #[test]
    fn parses_defaultable_config() {
        let config: CefariConfig = serde_json::from_str(
            r#"{
              "app": {
                "identifier": "dev.cefari.test",
                "display_name": "Test Cefari",
                "version": "1.2.3"
              },
              "deep_links": {
                "schemes": ["testapp"]
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
        .expect("config should parse");

        assert_eq!(
            config.app,
            AppConfig {
                identifier: "dev.cefari.test".to_owned(),
                display_name: "Test Cefari".to_owned(),
                version: "1.2.3".to_owned(),
            }
        );
        assert_eq!(config.deep_links.schemes, vec!["testapp"]);
        assert_eq!(config.browser, BrowserConfig::default());
        assert_eq!(config.daemon, DaemonConfig::default());
        assert_eq!(config.service, ServiceConfig::default());
        assert_eq!(
            match &config.workers.entries["thumbnailer"].target {
                WorkerTargetConfig::DenoSource(source) => &source.permissions.read,
                WorkerTargetConfig::Executable(_) => panic!("expected source worker target"),
            },
            &WorkerPermissionConfig::Allow(vec!["$appData/uploads".to_owned()])
        );
    }

    #[test]
    fn parses_browser_config() {
        let config: CefariConfig = serde_json::from_str(
            r#"{
              "browser": {
                "webgpu": true
              }
            }"#,
        )
        .expect("config should parse");

        assert_eq!(config.browser, BrowserConfig { webgpu: true });
    }

    #[test]
    fn parses_configured_daemon() {
        let config: CefariConfig = serde_json::from_str(
            r#"{
              "daemon": {
                "enabled": true,
                "executable": "daemon/example-daemon"
              }
            }"#,
        )
        .expect("config should parse");

        assert_eq!(
            config.daemon,
            DaemonConfig {
                enabled: true,
                executable: Some("daemon/example-daemon".to_owned()),
                native: Vec::new(),
            }
        );
    }

    #[test]
    fn parses_executable_worker_target() {
        let config: CefariConfig = serde_json::from_str(
            r#"{
              "workers": {
                "entries": {
                  "thumbnailer": {
                    "target": {
                      "kind": "executable",
                      "program": "workers/thumbnailer/thumbnailer"
                    },
                    "native": [
                      {
                        "id": "thumb-tool",
                        "target": "bin/thumb",
                        "path": "workers/thumbnailer/native/bin/thumb",
                        "executable": true
                      }
                    ]
                  }
                }
              }
            }"#,
        )
        .expect("config should parse");

        assert_eq!(
            match &config.workers.entries["thumbnailer"].target {
                WorkerTargetConfig::Executable(executable) => executable.program.as_str(),
                WorkerTargetConfig::DenoSource(_) => panic!("expected executable worker target"),
            },
            "workers/thumbnailer/thumbnailer"
        );
        assert_eq!(config.workers.entries["thumbnailer"].native.len(), 1);
        assert_eq!(
            config.workers.entries["thumbnailer"].native[0].id,
            "thumb-tool"
        );
        assert_eq!(
            config.workers.entries["thumbnailer"].native[0].path,
            "workers/thumbnailer/native/bin/thumb"
        );
        assert!(config.workers.entries["thumbnailer"].native[0].executable);
    }

    #[test]
    fn rejects_ambiguous_worker_target_config() {
        let error = serde_json::from_str::<CefariConfig>(
            r#"{
              "workers": {
                "entries": {
                  "thumbnailer": {
                    "target": {
                      "kind": "executable",
                      "entry": "workers/thumbnailer.ts",
                      "permissions": {},
                      "program": "workers/thumbnailer/thumbnailer"
                    }
                  }
                }
              }
            }"#,
        )
        .expect_err("ambiguous worker target should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let error = serde_json::from_str::<CefariConfig>(
            r#"{
              "app": {
                "identifier": "dev.cefari.test",
                "display_name": "Test Cefari",
                "unexpected": true
              }
            }"#,
        )
        .expect_err("unknown fields should be rejected");

        assert!(error.to_string().contains("unknown field"));
    }
}
