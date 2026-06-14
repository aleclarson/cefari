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

pub fn package_project(
    project_dir: &Path,
    release: bool,
    release_version: Option<&str>,
) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let package_dir = ProjectConfig::dist_dir(project_dir).join("package");

    ensure_build_artifacts(&build_dir, &project)?;
    let cef_resources_dir = cef::prepared_resources_dir(project_dir)?;
    verify_cef_package_payload(&build_dir, &project, &cef_resources_dir)?;
    fs::create_dir_all(&package_dir).with_context(|| {
        format!(
            "failed to create package directory at {}",
            package_dir.display()
        )
    })?;

    write_package_metadata(
        project_dir,
        &package_dir,
        &project,
        &build_dir,
        &cef_resources_dir,
        release,
        release_version,
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

fn verify_cef_package_payload(
    build_dir: &Path,
    project: &ProjectConfig,
    cef_resources_dir: &Path,
) -> Result<()> {
    let subprocess = build_dir
        .join("desktop")
        .join(desktop_executable_name(project));
    if !subprocess.is_file() {
        anyhow::bail!(
            "missing CEF subprocess executable {}; run cefari build first",
            subprocess.display()
        );
    }

    let archive_json = cef_resources_dir.join("archive.json");
    if !archive_json.is_file() {
        anyhow::bail!(
            "missing CEF archive metadata {}; run cefari build first",
            archive_json.display()
        );
    }

    if !has_payload_file(cef_resources_dir, Some("archive.json"))? {
        anyhow::bail!(
            "CEF resources directory {} contains no runtime payload files",
            cef_resources_dir.display()
        );
    }

    let locales_dir = cef_resources_dir.join("locales");
    if !locales_dir.is_dir() {
        anyhow::bail!(
            "missing CEF locales directory {}; package resources must include locales",
            locales_dir.display()
        );
    }
    if !has_payload_file(&locales_dir, None)? {
        anyhow::bail!(
            "CEF locales directory {} contains no locale files",
            locales_dir.display()
        );
    }

    let framework_dir = cef_resources_dir.join("Chromium Embedded Framework.framework");
    if framework_dir.exists() && !has_payload_file(&framework_dir, None)? {
        anyhow::bail!(
            "CEF framework directory {} contains no framework files",
            framework_dir.display()
        );
    }

    Ok(())
}

fn has_payload_file(dir: &Path, ignored_file_name: Option<&str>) -> Result<bool> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read CEF directory {}", dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read CEF entry in {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for {}", entry.path().display()))?;
        let is_ignored = ignored_file_name
            .is_some_and(|ignored| entry.file_name() == std::ffi::OsStr::new(ignored));
        if file_type.is_file() && !is_ignored {
            return Ok(true);
        }
        if file_type.is_dir() && has_payload_file(&entry.path(), ignored_file_name)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn write_package_metadata(
    project_dir: &Path,
    package_dir: &Path,
    project: &ProjectConfig,
    build_dir: &Path,
    cef_resources_dir: &Path,
    _release: bool,
    release_version: Option<&str>,
) -> Result<()> {
    let icon_path = package_icon_path(project_dir, package_dir, project)?;
    let tray_icon_path = package_tray_icon_path(project_dir, project)?;
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
        icons: vec![normalize(&icon_path)],
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

fn package_icon_path(
    project_dir: &Path,
    package_dir: &Path,
    project: &ProjectConfig,
) -> Result<PathBuf> {
    let Some(icon) = &project.app.icon else {
        return write_default_package_icon(package_dir);
    };

    required_project_file(project_dir, icon, "configured app icon")
}

fn package_tray_icon_path(project_dir: &Path, project: &ProjectConfig) -> Result<Option<PathBuf>> {
    if !project.capabilities.tray {
        return Ok(None);
    }

    let tray_icon = project.app.tray_icon.as_deref().ok_or_else(|| {
        anyhow::anyhow!("app.tray_icon is required when capabilities.tray is true")
    })?;
    required_project_file(project_dir, tray_icon, "configured tray icon").map(Some)
}

fn required_project_file(project_dir: &Path, relative_path: &str, label: &str) -> Result<PathBuf> {
    let path = project_dir.join(relative_path);
    if !path.is_file() {
        anyhow::bail!("{label} {} does not exist or is not a file", path.display());
    }

    path.canonicalize()
        .with_context(|| format!("failed to resolve {label} at {}", path.display()))
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
        tray_icon: project
            .capabilities
            .tray
            .then_some("tray-icon.png".to_owned()),
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

    let manifest_json = manifest.to_json()?;
    fs::write(package_dir.join("manifest.json"), manifest_json).with_context(|| {
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

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::project::ProjectConfig;

    use super::{PackageManifest, verify_cef_package_payload};

    #[test]
    fn verifies_cef_package_payload_contract() {
        let root = temp_dir("valid");
        let project = project_config();
        let build_dir = root.join("build");
        let cef_resources = root.join("cef");
        create_desktop_binary(&build_dir, "example-app");
        create_cef_resources(&cef_resources, true);

        verify_cef_package_payload(&build_dir, &project, &cef_resources)
            .expect("payload should verify");

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn rejects_cef_package_payload_without_locales() {
        let root = temp_dir("missing-locales");
        let project = project_config();
        let build_dir = root.join("build");
        let cef_resources = root.join("cef");
        create_desktop_binary(&build_dir, "example-app");
        create_cef_resources(&cef_resources, false);

        let error = verify_cef_package_payload(&build_dir, &project, &cef_resources)
            .expect_err("missing locales should fail verification");

        assert!(error.to_string().contains("missing CEF locales directory"));

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

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

    fn create_desktop_binary(build_dir: &Path, name: &str) {
        fs::create_dir_all(build_dir.join("desktop")).expect("desktop dir should exist");
        fs::write(build_dir.join("desktop").join(name), "desktop")
            .expect("desktop binary should be written");
    }

    fn create_cef_resources(resources: &Path, with_locales: bool) {
        fs::create_dir_all(resources).expect("CEF resources dir should exist");
        fs::write(resources.join("archive.json"), "{}").expect("archive should be written");
        fs::write(resources.join("libcef.fixture"), "fixture").expect("payload should be written");
        if with_locales {
            fs::create_dir_all(resources.join("locales")).expect("locales dir should exist");
            fs::write(resources.join("locales/en-US.pak"), "locale")
                .expect("locale should be written");
        }
    }

    fn project_config() -> ProjectConfig {
        toml::from_str(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"
tray_icon = "assets/tray-icon.png"

[capabilities]
tray = true

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
version = "1.2.3"
"#,
        )
        .expect("project should parse")
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-package-cef-{label}-{suffix}"))
    }
}
