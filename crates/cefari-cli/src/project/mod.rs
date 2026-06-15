use std::path::{Path, PathBuf};

mod deno;
mod loader;
mod schema;
mod validation;

pub const CONFIG_FILE_NAME: &str = "cefari.config.ts";

pub use deno::{DenoStatus, DenoVersion, deno_status};
pub use loader::LoadProjectError;
pub use schema::{
    DaemonConfig, FrontendConfig, PackageConfig, ProjectApp, ProjectCapability, ProjectConfig,
};
pub use validation::ProjectConfigValidationError;

impl ProjectConfig {
    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self, LoadProjectError> {
        let project_dir = path.as_ref();
        let config_path = project_dir.join(CONFIG_FILE_NAME);
        if !config_path.is_file() {
            return Err(LoadProjectError::Missing { path: config_path });
        }

        let deno = deno::deno_command();
        deno::warn_if_deno_is_older_than_expected(&deno)?;
        let loader_dir = loader::LoaderDir::create(project_dir)?;
        let output = loader::run_config_loader(&deno, project_dir, &config_path, &loader_dir)?;
        let project =
            serde_json::from_slice::<ProjectConfig>(&output.stdout).map_err(|source| {
                LoadProjectError::Parse {
                    path: config_path.clone(),
                    source,
                }
            })?;
        project
            .validate()
            .map_err(|source| LoadProjectError::Validate {
                path: config_path,
                source,
            })?;
        Ok(project)
    }

    #[must_use]
    pub fn build_dir(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("build")
    }

    #[must_use]
    pub fn dist_dir(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().join("dist")
    }

    #[must_use]
    pub fn tray_capability(&self) -> Option<&ProjectCapability> {
        self.capabilities
            .first()
            .map(|capability| match capability {
                ProjectCapability::Tray { .. } => capability,
            })
    }

    #[must_use]
    pub fn tray_icon(&self) -> Option<&str> {
        self.tray_capability()
            .and_then(|capability| match capability {
                ProjectCapability::Tray { icon } => icon.as_deref(),
            })
    }

    fn validate(&self) -> Result<(), ProjectConfigValidationError> {
        validation::validate_project_config(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ProjectConfig, ProjectConfigValidationError,
        deno::{DenoVersion, is_older_than_expected_deno, parse_deno_version},
    };

    fn project_config_json() -> &'static str {
        r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app",
    "icon": "assets/icon.png"
  },
  "capabilities": [
    {
      "type": "tray",
      "icon": "assets/tray-icon.png"
    }
  ],
  "frontend": {
    "dist": "frontend/dist",
    "devPort": 5173
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#
    }

    #[test]
    fn parses_project_config_json() {
        let project: ProjectConfig =
            serde_json::from_str(project_config_json()).expect("config should parse");
        project.validate().expect("config should validate");

        assert_eq!(project.app.project_name, "example-app");
        assert_eq!(project.app.name, "Example App");
        assert_eq!(project.app.icon.as_deref(), Some("assets/icon.png"));
        assert_eq!(project.tray_icon(), Some("assets/tray-icon.png"));
        assert_eq!(project.package.product_name, "Example App");
        assert_eq!(project.package.version, "1.2.3");
        assert_eq!(project.frontend.dev_port, 5173);
        assert!(project.frontend.build_command.is_none());
        assert!(project.frontend.dev_command.is_none());
    }

    #[test]
    fn parses_project_frontend_commands() {
        let project: ProjectConfig = serde_json::from_str(
            r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  },
  "frontend": {
    "dist": "frontend/dist",
    "buildCommand": ["npm", "--prefix", "frontend", "run", "build"],
    "devCommand": ["npm", "--prefix", "frontend", "run", "dev", "--", "--port", "{port}"],
    "devPort": 5174
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#,
        )
        .expect("config should parse");
        project.validate().expect("config should validate");

        assert_eq!(project.frontend.dev_port, 5174);
        assert!(project.tray_capability().is_none());
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
        let error = serde_json::from_str::<ProjectConfig>(
            r#"{
  "app": {
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  },
  "frontend": {
    "dist": "frontend/dist"
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#,
        )
        .expect_err("config without projectName should fail");

        assert!(error.to_string().contains("missing field `projectName`"));
    }

    #[test]
    fn validates_tray_icon_when_tray_capability_is_missing_icon() {
        let project: ProjectConfig = serde_json::from_str(
            r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  },
  "capabilities": [
    {
      "type": "tray"
    }
  ],
  "frontend": {
    "dist": "frontend/dist"
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#,
        )
        .expect("config should parse");

        let error = project
            .validate()
            .expect_err("tray without icon should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "capabilities[].icon",
                "is required for tray capabilities"
            )
        );
    }

    #[test]
    fn rejects_duplicate_tray_capabilities() {
        let project: ProjectConfig = serde_json::from_str(
            r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  },
  "capabilities": [
    {
      "type": "tray",
      "icon": "assets/tray-icon.png"
    },
    {
      "type": "tray",
      "icon": "assets/another-tray-icon.png"
    }
  ],
  "frontend": {
    "dist": "frontend/dist"
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#,
        )
        .expect("config should parse");

        let error = project
            .validate()
            .expect_err("duplicate tray capabilities should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "capabilities",
                "must not include more than one tray capability"
            )
        );
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
            let project: ProjectConfig = serde_json::from_str(&format!(
                r#"{{
  "app": {{
    "projectName": "{project_name}",
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  }},
  "frontend": {{
    "dist": "frontend/dist"
  }},
  "daemon": {{
    "entry": "daemon/main.ts"
  }},
  "package": {{
    "productName": "Example App",
    "version": "1.2.3"
  }}
}}"#
            ))
            .expect("schema should parse");
            let error = project
                .validate()
                .expect_err("config with invalid projectName should fail");

            assert_eq!(
                error,
                ProjectConfigValidationError::new(
                    "app.projectName",
                    "must match ^[a-z0-9-]+$ and cannot be empty"
                )
            );
        }
    }

    #[test]
    fn rejects_unknown_project_config_fields() {
        let error = serde_json::from_str::<ProjectConfig>(
            r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app",
    "extra": true
  },
  "frontend": {
    "dist": "frontend/dist"
  },
  "daemon": {
    "entry": "daemon/main.ts"
  },
  "package": {
    "productName": "Example App",
    "version": "1.2.3"
  }
}"#,
        )
        .expect_err("config with extra field should fail");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_blank_required_strings() {
        let mut project: ProjectConfig =
            serde_json::from_str(project_config_json()).expect("config should parse");
        project.app.name.clear();

        let error = project.validate().expect_err("blank name should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new("app.name", "must not be blank")
        );
    }

    #[test]
    fn rejects_absolute_paths() {
        let mut project: ProjectConfig =
            serde_json::from_str(project_config_json()).expect("config should parse");
        project.daemon.entry = "/tmp/main.ts".to_owned();

        let error = project.validate().expect_err("absolute path should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "daemon.entry",
                "must be a relative path inside the project"
            )
        );
    }

    #[test]
    fn rejects_empty_commands() {
        let mut project: ProjectConfig =
            serde_json::from_str(project_config_json()).expect("config should parse");
        project.frontend.build_command = Some(Vec::new());

        let error = project.validate().expect_err("empty command should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "frontend.buildCommand",
                "must contain at least one argument"
            )
        );
    }

    #[test]
    fn rejects_invalid_versions() {
        let mut project: ProjectConfig =
            serde_json::from_str(project_config_json()).expect("config should parse");
        project.package.version = "not a version".to_owned();

        let error = project.validate().expect_err("invalid version should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "package.version",
                "must be a valid semantic version"
            )
        );
    }

    #[test]
    fn parses_deno_version_output() {
        assert_eq!(
            parse_deno_version("deno 2.8.3 (stable, release)\nv8 14\n"),
            Some(DenoVersion {
                major: 2,
                minor: 8,
                patch: 3,
            })
        );
    }

    #[test]
    fn detects_older_deno_versions() {
        assert!(is_older_than_expected_deno(DenoVersion {
            major: 2,
            minor: 7,
            patch: 9,
        }));
        assert!(!is_older_than_expected_deno(DenoVersion {
            major: 2,
            minor: 8,
            patch: 0,
        }));
        assert!(!is_older_than_expected_deno(DenoVersion {
            major: 3,
            minor: 0,
            patch: 0,
        }));
    }
}
