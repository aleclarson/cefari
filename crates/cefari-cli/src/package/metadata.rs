use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{build::desktop_executable_name, project::ProjectConfig};

use super::{icons, normalize_path};

pub(super) fn write_package_metadata(
    project_dir: &Path,
    package_dir: &Path,
    project: &ProjectConfig,
    build_dir: &Path,
    cef_resources_dir: &Path,
    _release: bool,
    release_version: Option<&str>,
) -> Result<()> {
    let icon_path = icons::package_icon_path(project_dir, package_dir, project)?;
    let tray_icon_path = icons::package_tray_icon_path(project_dir, project)?;
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
    let mut resources = vec![
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
    ];
    if let Some(tray_icon_path) = tray_icon_path {
        resources.push(CargoPackagerResource {
            src: tray_icon_path,
            target: "tray-icon.png".into(),
        });
    }

    let metadata = CargoPackagerConfig {
        name: project.app.identifier.clone(),
        product_name: project.package.product_name.clone(),
        version: release_version
            .unwrap_or(&project.package.version)
            .to_owned(),
        identifier: Some(project.app.identifier.clone()),
        binaries_dir: Some(desktop_dir),
        icons: vec![normalize_path(&icon_path)],
        binaries: vec![CargoPackagerBinary {
            path: desktop_executable_name(project).into(),
            main: true,
        }],
        resources,
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
