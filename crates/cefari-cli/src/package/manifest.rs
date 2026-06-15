use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    build::{daemon_executable_name, desktop_executable_name},
    project::ProjectConfig,
};

use super::normalize_path;

pub(super) fn write_package_manifest(
    package_dir: &Path,
    project: &ProjectConfig,
    build_dir: &Path,
    cef_resources_dir: &Path,
) -> Result<()> {
    let manifest = PackageManifest {
        product_name: project.package.product_name.clone(),
        identifier: project.app.identifier.clone(),
        tray_icon: project
            .tray_capability()
            .map(|_| "tray-icon.png".to_owned()),
        desktop_binary: desktop_executable_name(project),
        frontend_dir: normalize_path(&build_dir.join("frontend")),
        daemon_dir: normalize_path(&build_dir.join("daemon")),
        daemon_executable: normalize_path(
            &build_dir
                .join("daemon")
                .join(daemon_executable_name(project)),
        ),
        cef_resources: normalize_path(cef_resources_dir),
        cef_archive_json: normalize_path(&cef_resources_dir.join("archive.json")),
    };

    let manifest_json = manifest.to_json()?;
    fs::write(package_dir.join("manifest.json"), manifest_json).with_context(|| {
        format!(
            "failed to write package manifest at {}",
            package_dir.join("manifest.json").display()
        )
    })
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
struct PackageManifest {
    product_name: String,
    identifier: String,
    tray_icon: Option<String>,
    desktop_binary: String,
    frontend_dir: String,
    daemon_dir: String,
    daemon_executable: String,
    cef_resources: String,
    cef_archive_json: String,
}

impl PackageManifest {
    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("failed to encode package manifest")
    }
}

#[cfg(test)]
mod tests {
    use super::PackageManifest;

    #[test]
    fn package_manifest_escapes_json_strings() {
        let manifest = PackageManifest {
            product_name: "Quoted \"App\" \\ Demo\nNext".to_owned(),
            identifier: "dev.cefari.quoted-app".to_owned(),
            tray_icon: Some("tray-icon.png".to_owned()),
            desktop_binary: "quoted-app".to_owned(),
            frontend_dir: "/tmp/frontend".to_owned(),
            daemon_dir: "/tmp/daemon".to_owned(),
            daemon_executable: "/tmp/daemon/quoted-app-daemon".to_owned(),
            cef_resources: "/tmp/cef".to_owned(),
            cef_archive_json: "/tmp/cef/archive.json".to_owned(),
        };

        let json = manifest.to_json().expect("manifest should serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("manifest should be valid JSON");

        assert_eq!(value["product_name"], "Quoted \"App\" \\ Demo\nNext");
    }
}
