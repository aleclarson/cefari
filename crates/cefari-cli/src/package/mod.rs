use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{cef, project::ProjectConfig};

mod cargo_packager;
mod icons;
mod manifest;
mod metadata;
mod verify;

pub fn package_project(
    project_dir: &Path,
    release: bool,
    release_version: Option<&str>,
) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let package_dir = ProjectConfig::dist_dir(project_dir).join("package");

    verify::ensure_build_artifacts(&build_dir, &project)?;
    let cef_resources_dir = cef::prepared_resources_dir(project_dir)?;
    verify::verify_cef_package_payload(&build_dir, &project, &cef_resources_dir)?;
    fs::create_dir_all(&package_dir).with_context(|| {
        format!(
            "failed to create package directory at {}",
            package_dir.display()
        )
    })?;

    metadata::write_package_metadata(
        project_dir,
        &package_dir,
        &project,
        &build_dir,
        &cef_resources_dir,
        release,
        release_version,
    )?;
    manifest::write_package_manifest(&package_dir, &project, &build_dir, &cef_resources_dir)?;

    println!("prepared package assembly at {}", package_dir.display());
    cargo_packager::run_cargo_packager_if_available(&package_dir)?;
    Ok(())
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
