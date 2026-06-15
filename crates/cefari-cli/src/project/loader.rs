use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use super::{CONFIG_FILE_NAME, validation::ProjectConfigValidationError};

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

export function tray(config) {
  return { type: 'tray', ...config };
}
";

pub(super) fn run_config_loader(
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

pub(super) struct LoaderDir {
    path: PathBuf,
    import_map: PathBuf,
}

impl LoaderDir {
    pub(super) fn create(project_dir: &Path) -> Result<Self, LoadProjectError> {
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
