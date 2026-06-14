use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use thiserror::Error;

pub const CONFIG_FILE_NAME: &str = "cefari.config.ts";
const DENO_ENV: &str = "CEFARI_DENO";
const EXPECTED_DENO_MAJOR: u64 = 2;
const EXPECTED_DENO_MINOR: u64 = 8;

const CONFIG_LOADER: &str = r#"
import { pathToFileURL } from "node:url";

function fail(message) {
  console.error(message);
  Deno.exit(1);
}

const configPath = Deno.args[0];
if (!configPath) {
  fail("missing config path");
}

let mod;
try {
  mod = await import(pathToFileURL(configPath).href);
} catch (error) {
  fail(`failed to import project config: ${error?.message ?? error}`);
}

if (!Object.hasOwn(mod, "default")) {
  fail("project config must have a default export");
}

const config = mod.default;
if (config === null || typeof config !== "object" || Array.isArray(config)) {
  fail("project config default export must be an object");
}

let json;
try {
  json = JSON.stringify(config);
} catch (error) {
  fail(`project config default export must be JSON-serializable: ${error?.message ?? error}`);
}

if (json === undefined) {
  fail("project config default export must be JSON-serializable");
}

console.log(json);
"#;

const CONFIG_API: &str = r"
export function defineConfig(config) {
  return config;
}
";

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub app: ProjectApp,
    #[serde(default)]
    pub capabilities: ProjectCapabilities,
    pub frontend: FrontendConfig,
    pub daemon: DaemonConfig,
    pub package: PackageConfig,
}

impl ProjectConfig {
    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self, LoadProjectError> {
        let project_dir = path.as_ref();
        let config_path = project_dir.join(CONFIG_FILE_NAME);
        if !config_path.is_file() {
            return Err(LoadProjectError::Missing { path: config_path });
        }

        let deno = deno_command();
        warn_if_deno_is_older_than_expected(&deno)?;
        let loader_dir = LoaderDir::create(project_dir)?;
        let output = run_config_loader(&deno, project_dir, &config_path, &loader_dir)?;
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

    fn validate(&self) -> Result<(), ProjectConfigValidationError> {
        validate_project_name(&self.app.project_name)?;
        validate_required_string("app.name", &self.app.name)?;
        validate_required_string("app.identifier", &self.app.identifier)?;
        validate_optional_relative_path("app.icon", self.app.icon.as_deref())?;
        validate_optional_relative_path("app.trayIcon", self.app.tray_icon.as_deref())?;
        if self.capabilities.tray && self.app.tray_icon.is_none() {
            return Err(ProjectConfigValidationError::new(
                "app.trayIcon",
                "is required when capabilities.tray is true",
            ));
        }
        validate_relative_path("frontend.dist", &self.frontend.dist)?;
        validate_command(
            "frontend.buildCommand",
            self.frontend.build_command.as_deref(),
        )?;
        validate_command("frontend.devCommand", self.frontend.dev_command.as_deref())?;
        if self.frontend.dev_port == 0 && self.frontend.dev_command.is_some() {
            return Err(ProjectConfigValidationError::new(
                "frontend.devPort",
                "must be greater than 0 when frontend.devCommand is configured",
            ));
        }
        validate_relative_path("daemon.entry", &self.daemon.entry)?;
        validate_required_string("package.productName", &self.package.product_name)?;
        validate_version("package.version", &self.package.version)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectApp {
    pub project_name: String,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tray_icon: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectCapabilities {
    pub tray: bool,
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

fn deno_command() -> PathBuf {
    std::env::var_os(DENO_ENV).map_or_else(|| PathBuf::from("deno"), PathBuf::from)
}

fn warn_if_deno_is_older_than_expected(deno: &Path) -> Result<(), LoadProjectError> {
    let output = Command::new(deno)
        .arg("--version")
        .output()
        .map_err(|source| LoadProjectError::DenoMissing { source })?;

    if !output.status.success() {
        return Err(LoadProjectError::DenoFailed {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(version) = parse_deno_version(&stdout)
        && is_older_than_expected_deno(version)
    {
        eprintln!(
            "warning: Cefari expects Deno {EXPECTED_DENO_MAJOR}.{EXPECTED_DENO_MINOR}+ to load {CONFIG_FILE_NAME}; found Deno {}.{}.{}",
            version.major, version.minor, version.patch
        );
    }

    Ok(())
}

#[must_use]
pub fn deno_status() -> DenoStatus {
    match Command::new(deno_command()).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_deno_version(&stdout).map_or_else(
                || DenoStatus::Unknown {
                    output: stdout.lines().next().unwrap_or_default().to_owned(),
                },
                |version| {
                    if is_older_than_expected_deno(version) {
                        DenoStatus::Older { version }
                    } else {
                        DenoStatus::Expected { version }
                    }
                },
            )
        }
        Ok(output) => DenoStatus::Failed {
            status: output.status.to_string(),
        },
        Err(_) => DenoStatus::Missing,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DenoStatus {
    Expected { version: DenoVersion },
    Older { version: DenoVersion },
    Unknown { output: String },
    Failed { status: String },
    Missing,
}

impl DenoStatus {
    #[must_use]
    pub fn doctor_message(&self) -> String {
        match self {
            Self::Expected { version } => format!("{version} OK"),
            Self::Older { version } => {
                format!("{version} older than expected; Cefari expects Deno 2.8+")
            }
            Self::Unknown { output } => format!("found, version unrecognized ({output})"),
            Self::Failed { status } => format!("failed ({status})"),
            Self::Missing => "missing".to_owned(),
        }
    }
}

fn run_config_loader(
    deno: &Path,
    project_dir: &Path,
    config_path: &Path,
    loader_dir: &LoaderDir,
) -> Result<std::process::Output, LoadProjectError> {
    let original_project_dir = project_dir.to_path_buf();
    let project_dir = project_dir
        .canonicalize()
        .map_err(|source| LoadProjectError::ReadRoot {
            path: project_dir.to_path_buf(),
            source,
        })?;
    let config_path = config_path
        .canonicalize()
        .map_err(|source| LoadProjectError::ReadRoot {
            path: config_path.to_path_buf(),
            source,
        })?;
    let original_loader_path = loader_dir.path.clone();
    let loader_path =
        loader_dir
            .path
            .canonicalize()
            .map_err(|source| LoadProjectError::CreateLoader {
                path: loader_dir.path.clone(),
                source,
            })?;
    let read_allowlist = format!(
        "{},{},{},{}",
        original_project_dir.display(),
        project_dir.display(),
        original_loader_path.display(),
        loader_path.display()
    );
    let mut child = Command::new(deno)
        .arg("run")
        .arg("--quiet")
        .arg(format!("--allow-read={read_allowlist}"))
        .arg("--allow-env")
        .arg("--import-map")
        .arg(&loader_dir.import_map)
        .arg("-")
        .arg(&config_path)
        .current_dir(project_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| LoadProjectError::Execute {
            path: config_path.clone(),
            source,
        })?;

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(CONFIG_LOADER.as_bytes())
        .map_err(|source| LoadProjectError::WriteLoader { source })?;

    let output = child
        .wait_with_output()
        .map_err(|source| LoadProjectError::Execute {
            path: config_path.clone(),
            source,
        })?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(LoadProjectError::DenoConfig {
            path: config_path.clone(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

fn validate_project_name(value: &str) -> Result<(), ProjectConfigValidationError> {
    if is_valid_project_name(value) {
        Ok(())
    } else {
        Err(ProjectConfigValidationError::new(
            "app.projectName",
            "must match ^[a-z0-9-]+$ and cannot be empty",
        ))
    }
}

fn is_valid_project_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_required_string(
    field: &'static str,
    value: &str,
) -> Result<(), ProjectConfigValidationError> {
    if value.trim().is_empty() {
        Err(ProjectConfigValidationError::new(
            field,
            "must not be blank",
        ))
    } else {
        Ok(())
    }
}

fn validate_optional_relative_path(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ProjectConfigValidationError> {
    value.map_or(Ok(()), |value| validate_relative_path(field, value))
}

fn validate_relative_path(
    field: &'static str,
    value: &str,
) -> Result<(), ProjectConfigValidationError> {
    validate_required_string(field, value)?;
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ProjectConfigValidationError::new(
            field,
            "must be a relative path inside the project",
        ));
    }
    Ok(())
}

fn validate_command(
    field: &'static str,
    value: Option<&[String]>,
) -> Result<(), ProjectConfigValidationError> {
    let Some(command) = value else {
        return Ok(());
    };
    if command.is_empty() {
        return Err(ProjectConfigValidationError::new(
            field,
            "must contain at least one argument",
        ));
    }
    if command.iter().any(|argument| argument.trim().is_empty()) {
        return Err(ProjectConfigValidationError::new(
            field,
            "must contain only non-blank arguments",
        ));
    }
    Ok(())
}

fn validate_version(field: &'static str, value: &str) -> Result<(), ProjectConfigValidationError> {
    validate_required_string(field, value)?;
    if semver::Version::parse(value).is_err() {
        return Err(ProjectConfigValidationError::new(
            field,
            "must be a valid semantic version",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectConfigValidationError {
    field: &'static str,
    message: &'static str,
}

impl ProjectConfigValidationError {
    fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}

impl std::fmt::Display for ProjectConfigValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.field, self.message)
    }
}

impl std::error::Error for ProjectConfigValidationError {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DenoVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl std::fmt::Display for DenoVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn parse_deno_version(output: &str) -> Option<DenoVersion> {
    let line = output.lines().find(|line| line.starts_with("deno "))?;
    let version = line.strip_prefix("deno ")?;
    let mut parts = version.split_whitespace().next()?.split('.');
    Some(DenoVersion {
        major: parts.next()?.parse().ok()?,
        minor: parts.next()?.parse().ok()?,
        patch: parts.next()?.parse().ok()?,
    })
}

fn is_older_than_expected_deno(version: DenoVersion) -> bool {
    (version.major, version.minor) < (EXPECTED_DENO_MAJOR, EXPECTED_DENO_MINOR)
}

struct LoaderDir {
    path: PathBuf,
    import_map: PathBuf,
}

impl LoaderDir {
    fn create(project_dir: &Path) -> Result<Self, LoadProjectError> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cefari-config-loader-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&path).map_err(|source| LoadProjectError::CreateLoader {
            path: path.clone(),
            source,
        })?;

        let api = path.join("cefari-cli-config-api.js");
        fs::write(&api, CONFIG_API).map_err(|source| LoadProjectError::CreateLoader {
            path: api.clone(),
            source,
        })?;

        let import_map = path.join("import_map.json");
        let import_map_json = serde_json::json!({
            "imports": {
                "@cefari/cli": "./cefari-cli-config-api.js",
            }
        });
        fs::write(&import_map, import_map_json.to_string()).map_err(|source| {
            LoadProjectError::CreateLoader {
                path: import_map.clone(),
                source,
            }
        })?;

        let _ = project_dir;
        Ok(Self { path, import_map })
    }
}

impl Drop for LoaderDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug, Error)]
pub enum LoadProjectError {
    #[error("project config not found at {path}")]
    Missing { path: PathBuf },

    #[error("Deno is required to load {CONFIG_FILE_NAME} but was not found")]
    DenoMissing { source: io::Error },

    #[error("failed to resolve project directory at {path}")]
    ReadRoot { path: PathBuf, source: io::Error },

    #[error("failed to check Deno version: {status}: {stderr}")]
    DenoFailed { status: String, stderr: String },

    #[error("failed to create config loader at {path}")]
    CreateLoader { path: PathBuf, source: io::Error },

    #[error("failed to write config loader to Deno stdin")]
    WriteLoader { source: io::Error },

    #[error("failed to execute project config at {path}")]
    Execute { path: PathBuf, source: io::Error },

    #[error("failed to execute project config at {path}: {status}: {stderr}")]
    DenoConfig {
        path: PathBuf,
        status: String,
        stderr: String,
    },

    #[error("failed to parse project config at {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to validate project config at {path}: {source}")]
    Validate {
        path: PathBuf,
        source: ProjectConfigValidationError,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        DenoVersion, ProjectConfig, ProjectConfigValidationError, is_older_than_expected_deno,
        parse_deno_version,
    };

    fn project_config_json() -> &'static str {
        r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app",
    "icon": "assets/icon.png",
    "trayIcon": "assets/tray-icon.png"
  },
  "capabilities": {
    "tray": true
  },
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
        assert!(project.capabilities.tray);
        assert_eq!(project.app.icon.as_deref(), Some("assets/icon.png"));
        assert_eq!(
            project.app.tray_icon.as_deref(),
            Some("assets/tray-icon.png")
        );
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
        assert!(!project.capabilities.tray);
        assert!(project.app.tray_icon.is_none());
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
    fn validates_tray_icon_when_tray_capability_is_enabled() {
        let project: ProjectConfig = serde_json::from_str(
            r#"{
  "app": {
    "projectName": "example-app",
    "name": "Example App",
    "identifier": "dev.cefari.example-app"
  },
  "capabilities": {
    "tray": true
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
        .expect("config should parse");

        let error = project
            .validate()
            .expect_err("tray without icon should fail");

        assert_eq!(
            error,
            ProjectConfigValidationError::new(
                "app.trayIcon",
                "is required when capabilities.tray is true"
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
