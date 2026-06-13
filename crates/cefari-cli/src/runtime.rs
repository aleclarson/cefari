use std::{env, fs, path::PathBuf, process::Command};

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, RuntimePaths};

use crate::build::{workspace_manifest, workspace_target_dir};

const DESKTOP_RUNTIME_ENV: &str = "CEFARI_DESKTOP_RUNTIME";

pub(crate) fn desktop_crate_binary_name() -> &'static str {
    if cfg!(windows) {
        "cefari-desktop.exe"
    } else {
        "cefari-desktop"
    }
}

pub(crate) fn resolve_desktop_runtime(release: bool) -> Result<PathBuf> {
    if let Some(path) = env::var_os(DESKTOP_RUNTIME_ENV) {
        return validate_runtime(PathBuf::from(path), DESKTOP_RUNTIME_ENV);
    }

    if running_from_workspace_target_dir()? {
        return build_desktop_runtime(release);
    }

    if let Some(path) = bundled_desktop_runtime()? {
        return Ok(path);
    }

    if let Some(path) = cached_desktop_runtime() {
        return Ok(path);
    }

    if workspace_manifest().is_file() {
        return build_desktop_runtime(release);
    }

    anyhow::bail!(
        "cefari-desktop runtime was not found; install a Cefari CLI distribution that bundles \
         cefari-desktop or set {DESKTOP_RUNTIME_ENV} to a prebuilt runtime"
    );
}

fn validate_runtime(path: PathBuf, source: &str) -> Result<PathBuf> {
    if path.is_file() {
        Ok(path)
    } else {
        anyhow::bail!(
            "{source} points to missing cefari-desktop runtime {}",
            path.display()
        );
    }
}

fn build_desktop_runtime(release: bool) -> Result<PathBuf> {
    let mut command = Command::new("cargo");
    configure_desktop_build_command(&mut command);
    if release {
        command.arg("--release");
    }

    let status = command
        .status()
        .context("failed to run cargo build for cefari-desktop")?;

    if !status.success() {
        anyhow::bail!("cargo build -p cefari-desktop failed with status {status}");
    }

    let runtime = workspace_target_dir(release).join(desktop_crate_binary_name());
    validate_runtime(runtime, "cargo build output")
}

pub(crate) fn configure_desktop_build_command(command: &mut Command) {
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(workspace_manifest())
        .arg("-p")
        .arg("cefari-desktop");
}

fn running_from_workspace_target_dir() -> Result<bool> {
    let workspace = workspace_manifest()
        .parent()
        .context("workspace manifest should have a parent")?
        .to_path_buf();
    if !workspace.join("Cargo.toml").is_file() {
        return Ok(false);
    }

    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    let current_exe = fs::canonicalize(&current_exe).unwrap_or(current_exe);
    let target_dir = workspace.join("target");
    let target_dir = fs::canonicalize(&target_dir).unwrap_or(target_dir);
    Ok(current_exe.starts_with(target_dir))
}

fn bundled_desktop_runtime() -> Result<Option<PathBuf>> {
    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    let Some(exe_dir) = current_exe.parent() else {
        return Ok(None);
    };

    let candidates = [
        exe_dir.join(desktop_crate_binary_name()),
        exe_dir
            .join("cefari-runtime")
            .join(desktop_crate_binary_name()),
        exe_dir
            .parent()
            .map(|parent| {
                parent
                    .join("lib")
                    .join("cefari")
                    .join(desktop_crate_binary_name())
            })
            .unwrap_or_else(|| exe_dir.join("missing")),
        exe_dir
            .parent()
            .map(|parent| {
                parent
                    .join("libexec")
                    .join("cefari")
                    .join(desktop_crate_binary_name())
            })
            .unwrap_or_else(|| exe_dir.join("missing")),
    ];

    Ok(candidates.into_iter().find(|path| path.is_file()))
}

fn cached_desktop_runtime() -> Option<PathBuf> {
    let Ok(paths) = RuntimePaths::resolve(&AppIdentity::cefari()) else {
        return None;
    };
    let runtime = paths
        .cache_dir
        .join("runtimes")
        .join(env!("CARGO_PKG_VERSION"))
        .join(runtime_target())
        .join(desktop_crate_binary_name());

    runtime.is_file().then_some(runtime)
}

fn runtime_target() -> &'static str {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("windows", "aarch64") => "windows-aarch64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::runtime_target;

    #[test]
    fn runtime_target_is_known_for_supported_hosts() {
        assert_ne!(runtime_target(), "unknown");
    }
}
