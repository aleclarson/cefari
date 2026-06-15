use serde::Deserialize;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub app: ProjectApp,
    #[serde(default)]
    pub capabilities: Vec<ProjectCapability>,
    pub frontend: FrontendConfig,
    pub daemon: DaemonConfig,
    pub package: PackageConfig,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectApp {
    pub project_name: String,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", deny_unknown_fields, rename_all = "camelCase")]
pub enum ProjectCapability {
    Tray {
        #[serde(default)]
        icon: Option<String>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FrontendConfig {
    pub dist: String,
    #[serde(default)]
    pub build_command: Option<Vec<String>>,
    #[serde(default)]
    pub dev_command: Option<Vec<String>>,
    #[serde(default = "default_frontend_dev_port")]
    pub dev_port: u16,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DaemonConfig {
    pub entry: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackageConfig {
    pub product_name: String,
    pub version: String,
}

fn default_frontend_dev_port() -> u16 {
    5173
}
