use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    build::{daemon_executable_name, desktop_executable_name},
    cef,
    project::ProjectConfig,
    run_process, tool_available,
};

pub fn package_project(project_dir: &Path, release: bool) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let package_dir = ProjectConfig::dist_dir(project_dir).join("package");

    ensure_build_artifacts(&build_dir, &project)?;
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

fn ensure_build_artifacts(build_dir: &Path, project: &ProjectConfig) -> Result<()> {
    let required = [
        build_dir.join("frontend/index.html"),
        build_dir.join("daemon/main.ts"),
        build_dir
            .join("daemon")
            .join(daemon_executable_name(project)),
        build_dir
            .join("desktop")
            .join(desktop_executable_name(project)),
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
    _release: bool,
) -> Result<()> {
    let icon_path = write_default_package_icon(package_dir)?;
    let frontend_dir = build_dir.join("frontend").canonicalize().with_context(|| {
        format!(
            "failed to resolve frontend resources at {}",
            build_dir.join("frontend").display()
        )
    })?;
    let daemon_dir = build_dir.join("daemon").canonicalize().with_context(|| {
        format!(
            "failed to resolve daemon resources at {}",
            build_dir.join("daemon").display()
        )
    })?;
    let desktop_dir = build_dir.join("desktop").canonicalize().with_context(|| {
        format!(
            "failed to resolve desktop binary resources at {}",
            build_dir.join("desktop").display()
        )
    })?;
    let cef_resources_dir = cef_resources_dir.canonicalize().with_context(|| {
        format!(
            "failed to resolve CEF resources at {}",
            cef_resources_dir.display()
        )
    })?;
    let metadata = CargoPackagerConfig {
        name: project.app.identifier.clone(),
        product_name: project.package.product_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        identifier: Some(project.app.identifier.clone()),
        binaries_dir: Some(desktop_dir),
        icons: vec![normalize(&icon_path)],
        binaries: vec![CargoPackagerBinary {
            path: desktop_executable_name(project).into(),
            main: true,
        }],
        resources: vec![
            CargoPackagerResource {
                src: frontend_dir,
                target: "frontend".into(),
            },
            CargoPackagerResource {
                src: daemon_dir,
                target: "daemon".into(),
            },
            CargoPackagerResource {
                src: cef_resources_dir,
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
    icons: Vec<String>,
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

fn write_default_package_icon(package_dir: &Path) -> Result<PathBuf> {
    let icons_dir = package_dir.join("icons");
    fs::create_dir_all(&icons_dir).with_context(|| {
        format!(
            "failed to create package icons directory at {}",
            icons_dir.display()
        )
    })?;

    let icon_path = icons_dir.join("cefari.png");
    fs::write(&icon_path, DEFAULT_PACKAGE_ICON_PNG)
        .with_context(|| format!("failed to write package icon at {}", icon_path.display()))?;
    icon_path
        .canonicalize()
        .with_context(|| format!("failed to resolve package icon at {}", icon_path.display()))
}

// 128x128 RGBA PNG fallback used when a project has not provided package icons.
const DEFAULT_PACKAGE_ICON_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x08, 0x06, 0x00, 0x00, 0x00, 0xc3, 0x3e, 0x61,
    0xcb, 0x00, 0x00, 0x00, 0xf2, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0xed, 0xd2, 0x31, 0x0d, 0x00,
    0x00, 0x08, 0xc0, 0x30, 0x9c, 0x20, 0x11, 0x21, 0x88, 0x06, 0x1b, 0x24, 0xf4, 0x98, 0x81, 0xa5,
    0x91, 0xd5, 0xa3, 0xbf, 0x85, 0x09, 0x00, 0x18, 0x01, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00,
    0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01,
    0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04,
    0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10,
    0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40,
    0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00,
    0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00,
    0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00,
    0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02,
    0x00, 0x00, 0x13, 0x00, 0x30, 0x02, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02,
    0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08,
    0x00, 0x01, 0x20, 0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20,
    0x00, 0x04, 0x80, 0x00, 0x10, 0x00, 0x02, 0x40, 0x00, 0x08, 0x00, 0x01, 0x20, 0x00, 0x04, 0x80,
    0x00, 0x10, 0x00, 0x02, 0x40, 0x37, 0x5a, 0xac, 0xe8, 0x07, 0xdb, 0x00, 0x81, 0x83, 0x86, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn write_package_manifest(
    package_dir: &Path,
    project: &ProjectConfig,
    build_dir: &Path,
    cef_resources_dir: &Path,
) -> Result<()> {
    let manifest = PackageManifest {
        product_name: project.package.product_name.clone(),
        identifier: project.app.identifier.clone(),
        desktop_binary: desktop_executable_name(project),
        frontend_dir: normalize(&build_dir.join("frontend")),
        daemon_dir: normalize(&build_dir.join("daemon")),
        daemon_executable: normalize(
            &build_dir
                .join("daemon")
                .join(daemon_executable_name(project)),
        ),
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
    let use_cargo_subcommand = !tool_available("cargo-packager") && tool_available("cargo");
    if !tool_available("cargo-packager") && !use_cargo_subcommand {
        println!("cargo-packager not found; skipped native package invocation");
        return Ok(());
    }

    let package_dir = package_dir.canonicalize().with_context(|| {
        format!(
            "failed to resolve package directory at {}",
            package_dir.display()
        )
    })?;
    let config = package_dir.join("cargo-packager.toml");
    let output_dir = package_dir.join("output");
    let mut command = if use_cargo_subcommand {
        let mut command = Command::new("cargo");
        command.arg("packager");
        command
    } else {
        Command::new("cargo-packager")
    };
    command
        .arg("--config")
        .arg(&config)
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
