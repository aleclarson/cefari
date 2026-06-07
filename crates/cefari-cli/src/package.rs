use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    build::{daemon_executable_name, workspace_manifest},
    cef,
    project::ProjectConfig,
    run_process, tool_available,
};

pub fn package_project(project_dir: &Path, release: bool) -> Result<()> {
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

    write_package_metadata(
        &package_dir,
        &project,
        &build_dir,
        &cef_resources_dir,
        release,
    )?;
    write_package_manifest(&package_dir, &project, &build_dir, &cef_resources_dir)?;

    println!("prepared package assembly at {}", package_dir.display());
    run_cargo_packager_if_available(&package_dir)?;
    Ok(())
}

fn ensure_build_artifacts(build_dir: &Path) -> Result<()> {
    let required = [
        build_dir.join("frontend/index.html"),
        build_dir.join("daemon/main.ts"),
        build_dir.join("daemon").join(daemon_executable_name()),
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
    build_dir: &Path,
    cef_resources_dir: &Path,
    release: bool,
) -> Result<()> {
    let metadata = CargoPackagerConfig {
        name: project.app.identifier.clone(),
        product_name: project.package.product_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        identifier: Some(project.app.identifier.clone()),
        binaries_dir: Some(workspace_target_dir(release)),
        binaries: vec![CargoPackagerBinary {
            path: desktop_binary_name().into(),
            main: true,
        }],
        resources: vec![
            CargoPackagerResource {
                src: build_dir.join("frontend"),
                target: "frontend".into(),
            },
            CargoPackagerResource {
                src: build_dir.join("daemon"),
                target: "daemon".into(),
            },
            CargoPackagerResource {
                src: cef_resources_dir.to_path_buf(),
                target: "cef".into(),
            },
        ],
    };
    let metadata =
        toml::to_string_pretty(&metadata).context("failed to encode package metadata")?;

    fs::write(package_dir.join("cargo-packager.toml"), metadata).with_context(|| {
        format!(
            "failed to write package metadata at {}",
            package_dir.join("cargo-packager.toml").display()
        )
    })
}

#[derive(Debug, Serialize)]
struct CargoPackagerConfig {
    name: String,
    product_name: String,
    version: String,
    identifier: Option<String>,
    binaries_dir: Option<PathBuf>,
    binaries: Vec<CargoPackagerBinary>,
    resources: Vec<CargoPackagerResource>,
}

#[derive(Debug, Serialize)]
struct CargoPackagerBinary {
    path: PathBuf,
    main: bool,
}

#[derive(Debug, Serialize)]
struct CargoPackagerResource {
    src: PathBuf,
    target: PathBuf,
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
        daemon_executable: normalize(&build_dir.join("daemon").join(daemon_executable_name())),
        cef_resources: normalize(cef_resources_dir),
        cef_archive_json: normalize(&cef_resources_dir.join("archive.json")),
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
    daemon_executable: String,
    cef_resources: String,
    cef_archive_json: String,
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
  "daemon_executable": "{}",
  "cef_resources": "{}",
  "cef_archive_json": "{}"
}}
"#,
            self.product_name,
            self.identifier,
            self.desktop_binary,
            self.frontend_dir,
            self.daemon_dir,
            self.daemon_executable,
            self.cef_resources,
            self.cef_archive_json
        )
    }
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn workspace_target_dir(release: bool) -> PathBuf {
    workspace_manifest()
        .parent()
        .expect("workspace manifest should have a parent")
        .join(if release {
            "target/release"
        } else {
            "target/debug"
        })
}

fn desktop_binary_name() -> &'static str {
    if cfg!(windows) {
        "cefari-desktop.exe"
    } else {
        "cefari-desktop"
    }
}
