use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, de};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub app: ProjectApp,
    pub frontend: FrontendConfig,
    pub daemon: DaemonConfig,
    pub package: PackageConfig,
}

impl ProjectConfig {
    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self, LoadProjectError> {
        let manifest_path = path.as_ref().join("cefari.toml");
        let contents =
            fs::read_to_string(&manifest_path).map_err(|source| match source.kind() {
                io::ErrorKind::NotFound => LoadProjectError::Missing {
                    path: manifest_path.clone(),
                },
                _ => LoadProjectError::Read {
                    path: manifest_path.clone(),
                    source,
                },
            })?;

        toml::from_str(&contents).map_err(|source| LoadProjectError::Parse {
            path: manifest_path,
            source,
        })
    }

    #[must_use]
    pub fn build_dir(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("build")
    }

    #[must_use]
    pub fn dist_dir(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("dist")
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectApp {
    #[serde(deserialize_with = "deserialize_project_name")]
    pub project_name: String,
    pub name: String,
    pub identifier: String,
    pub tray_icon: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct DaemonConfig {
    pub entry: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageConfig {
    pub product_name: String,
    pub version: String,
}

fn default_frontend_dev_port() -> u16 {
    5173
}

fn deserialize_project_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    if is_valid_project_name(&value) {
        Ok(value)
    } else {
        Err(de::Error::custom(
            "project_name must match ^[a-z0-9-]+$ and cannot be empty",
        ))
    }
}

fn is_valid_project_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[derive(Debug, Error)]
pub enum LoadProjectError {
    #[error("project manifest not found at {path}")]
    Missing { path: PathBuf },

    #[error("failed to read project manifest at {path}")]
    Read { path: PathBuf, source: io::Error },

    #[error("failed to parse project manifest at {path}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::ProjectConfig;

    #[test]
    fn parses_project_manifest() {
        let project: ProjectConfig = toml::from_str(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"
icon = "assets/icon.png"

[frontend]
dist = "frontend/dist"
dev_port = 5173

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect("manifest should parse");

        assert_eq!(project.app.project_name, "example-app");
        assert_eq!(project.app.name, "Example App");
        assert_eq!(project.app.tray_icon, "assets/tray-icon.png");
        assert_eq!(project.app.icon.as_deref(), Some("assets/icon.png"));
        assert_eq!(project.package.product_name, "Example App");
        assert_eq!(project.package.version, "1.2.3");
        assert_eq!(project.frontend.dev_port, 5173);
        assert!(project.frontend.build_command.is_none());
        assert!(project.frontend.dev_command.is_none());
    }

    #[test]
    fn parses_project_frontend_commands() {
        let project: ProjectConfig = toml::from_str(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"

[frontend]
dist = "frontend/dist"
build_command = ["npm", "--prefix", "frontend", "run", "build"]
dev_command = ["npm", "--prefix", "frontend", "run", "dev", "--", "--port", "{port}"]
dev_port = 5174

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect("manifest should parse");

        assert_eq!(project.frontend.dev_port, 5174);
        assert_eq!(
            project.frontend.build_command.as_deref(),
            Some(
                &[
                    "npm".to_owned(),
                    "--prefix".to_owned(),
                    "frontend".to_owned(),
                    "run".to_owned(),
                    "build".to_owned(),
                ][..]
            )
        );
    }

    #[test]
    fn rejects_missing_project_name() {
        let error = toml::from_str::<ProjectConfig>(
            r#"[app]
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect_err("manifest without project_name should fail");

        assert!(error.to_string().contains("missing field `project_name`"));
    }

    #[test]
    fn rejects_missing_tray_icon() {
        let error = toml::from_str::<ProjectConfig>(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect_err("manifest without tray_icon should fail");

        assert!(error.to_string().contains("missing field `tray_icon`"));
    }

    #[test]
    fn rejects_invalid_project_names() {
        for project_name in [
            "",
            "Example-App",
            "example_app",
            "example app",
            "example.app",
            "example/app",
        ] {
            let error = toml::from_str::<ProjectConfig>(&format!(
                r#"[app]
project_name = "{project_name}"
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#
            ))
            .expect_err("manifest with invalid project_name should fail");

            assert!(
                error
                    .to_string()
                    .contains("project_name must match ^[a-z0-9-]+$")
            );
        }
    }

    #[test]
    fn rejects_unknown_project_manifest_fields() {
        let error = toml::from_str::<ProjectConfig>(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"
extra = true

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect_err("manifest with extra field should fail");

        assert!(error.to_string().contains("unknown field"));
    }
}
