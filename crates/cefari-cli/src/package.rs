use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result};

use crate::{cef, project::ProjectConfig, run_process, tool_available};

pub fn package_project(project_dir: &Path) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let package_dir = ProjectConfig::dist_dir(project_dir).join("package");

    ensure_build_artifacts(&build_dir)?;
    let cef_resources_dir = cef::prepared_resources_dir(project_dir)?;
    fs::create_dir_all(&package_dir).with_context(|| {
        format!(
            "failed to create package directory at {}",
            package_dir.display()
        )
    })?;

    write_package_metadata(&package_dir, &project, &cef_resources_dir)?;
    write_package_manifest(&package_dir, &project, &build_dir, &cef_resources_dir)?;

    println!("prepared package assembly at {}", package_dir.display());
    run_cargo_packager_if_available(&package_dir)?;
    Ok(())
}

fn ensure_build_artifacts(build_dir: &Path) -> Result<()> {
    let required = [
        build_dir.join("frontend/index.html"),
        build_dir.join("daemon/main.ts"),
    ];

    for artifact in required {
        if !artifact.exists() {
            anyhow::bail!(
                "missing build artifact {}; run cefari build first",
                artifact.display()
            );
        }
    }

    Ok(())
}

fn write_package_metadata(
    package_dir: &Path,
    project: &ProjectConfig,
    cef_resources_dir: &Path,
) -> Result<()> {
    let metadata = format!(
        r#"[package]
product_name = "{}"
identifier = "{}"

[resources]
frontend = "build/frontend"
daemon = "build/daemon"
cef = "{}"
"#,
        project.package.product_name,
        project.app.identifier,
        normalize(cef_resources_dir)
    );

    fs::write(package_dir.join("cargo-packager.toml"), metadata).with_context(|| {
        format!(
            "failed to write package metadata at {}",
            package_dir.join("cargo-packager.toml").display()
        )
    })
}

fn write_package_manifest(
    package_dir: &Path,
    project: &ProjectConfig,
    build_dir: &Path,
    cef_resources_dir: &Path,
) -> Result<()> {
    let manifest = PackageManifest {
        product_name: project.package.product_name.clone(),
        identifier: project.app.identifier.clone(),
        desktop_binary: "cefari-desktop".to_owned(),
        frontend_dir: normalize(&build_dir.join("frontend")),
        daemon_dir: normalize(&build_dir.join("daemon")),
        cef_resources: normalize(cef_resources_dir),
    };

    fs::write(package_dir.join("manifest.json"), manifest.to_json()).with_context(|| {
        format!(
            "failed to write package manifest at {}",
            package_dir.join("manifest.json").display()
        )
    })
}

fn run_cargo_packager_if_available(package_dir: &Path) -> Result<()> {
    if !tool_available("cargo-packager") {
        println!("cargo-packager not found; skipped native package invocation");
        return Ok(());
    }

    let output_dir = package_dir.join("output");
    let mut command = Command::new("cargo-packager");
    command
        .arg("--config")
        .arg(package_dir.join("cargo-packager.toml"))
        .arg("--out-dir")
        .arg(&output_dir);

    run_process(&mut command, "cargo-packager")?;
    println!("created native packages at {}", output_dir.display());
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PackageManifest {
    product_name: String,
    identifier: String,
    desktop_binary: String,
    frontend_dir: String,
    daemon_dir: String,
    cef_resources: String,
}

impl PackageManifest {
    fn to_json(&self) -> String {
        format!(
            r#"{{
  "product_name": "{}",
  "identifier": "{}",
  "desktop_binary": "{}",
  "frontend_dir": "{}",
  "daemon_dir": "{}",
  "cef_resources": "{}"
}}
"#,
            self.product_name,
            self.identifier,
            self.desktop_binary,
            self.frontend_dir,
            self.daemon_dir,
            self.cef_resources
        )
    }
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
