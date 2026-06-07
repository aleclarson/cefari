use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
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
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectApp {
    pub name: String,
    pub identifier: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendConfig {
    pub dist: String,
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
name = "Example App"
identifier = "dev.cefari.example-app"

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
"#,
        )
        .expect("manifest should parse");

        assert_eq!(project.app.name, "Example App");
        assert_eq!(project.package.product_name, "Example App");
    }

    #[test]
    fn rejects_unknown_project_manifest_fields() {
        let error = toml::from_str::<ProjectConfig>(
            r#"[app]
name = "Example App"
identifier = "dev.cefari.example-app"
extra = true

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
"#,
        )
        .expect_err("manifest with extra field should fail");

        assert!(error.to_string().contains("unknown field"));
    }
}
