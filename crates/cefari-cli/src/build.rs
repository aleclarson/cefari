use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result};

use crate::project::ProjectConfig;

pub fn build_project(project_dir: &Path) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let frontend_out = build_dir.join("frontend");
    let daemon_out = build_dir.join("daemon");

    fs::create_dir_all(&frontend_out).with_context(|| {
        format!(
            "failed to create frontend build directory at {}",
            frontend_out.display()
        )
    })?;
    fs::create_dir_all(&daemon_out).with_context(|| {
        format!(
            "failed to create daemon build directory at {}",
            daemon_out.display()
        )
    })?;

    build_frontend(project_dir, &project, &frontend_out)?;
    build_daemon(project_dir, &project, &daemon_out)?;
    build_desktop()?;

    println!("built Cefari project at {}", project_dir.display());
    Ok(())
}

fn build_frontend(project_dir: &Path, project: &ProjectConfig, output_dir: &Path) -> Result<()> {
    let source = project_dir.join("frontend/index.html");
    let output = output_dir.join("index.html");
    fs::copy(&source, &output).with_context(|| {
        format!(
            "failed to copy frontend entry from {} to {}",
            source.display(),
            output.display()
        )
    })?;

    let configured_dist = project_dir.join(&project.frontend.dist);
    fs::create_dir_all(&configured_dist).with_context(|| {
        format!(
            "failed to create configured frontend dist at {}",
            configured_dist.display()
        )
    })?;
    fs::copy(&source, configured_dist.join("index.html")).with_context(|| {
        format!(
            "failed to copy frontend entry into configured dist at {}",
            configured_dist.display()
        )
    })?;

    Ok(())
}

fn build_daemon(project_dir: &Path, project: &ProjectConfig, output_dir: &Path) -> Result<()> {
    let source = project_dir.join(&project.daemon.entry);
    let output = output_dir.join("main.ts");
    fs::copy(&source, &output).with_context(|| {
        format!(
            "failed to copy daemon entry from {} to {}",
            source.display(),
            output.display()
        )
    })?;
    Ok(())
}

fn build_desktop() -> Result<()> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(workspace_manifest())
        .arg("-p")
        .arg("cefari-desktop")
        .status()
        .context("failed to run cargo build for cefari-desktop")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("cargo build -p cefari-desktop failed with status {status}");
    }
}

fn workspace_manifest() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("cefari-cli should live under crates/cefari-cli")
        .join("Cargo.toml")
}

#[cfg(test)]
mod tests {
    use super::workspace_manifest;

    #[test]
    fn resolves_workspace_manifest() {
        assert!(workspace_manifest().ends_with("Cargo.toml"));
        assert!(workspace_manifest().exists());
    }
}
