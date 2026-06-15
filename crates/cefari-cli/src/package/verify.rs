use std::{ffi::OsStr, fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    build::{daemon_executable_name, desktop_executable_name},
    project::ProjectConfig,
};

pub(super) fn ensure_build_artifacts(build_dir: &Path, project: &ProjectConfig) -> Result<()> {
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

pub(super) fn verify_cef_package_payload(
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
        let is_ignored =
            ignored_file_name.is_some_and(|ignored| entry.file_name() == OsStr::new(ignored));
        if file_type.is_file() && !is_ignored {
            return Ok(true);
        }
        if file_type.is_dir() && has_payload_file(&entry.path(), ignored_file_name)? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::project::{
        DaemonConfig, FrontendConfig, PackageConfig, ProjectApp, ProjectCapability, ProjectConfig,
    };

    use super::verify_cef_package_payload;

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
        ProjectConfig {
            app: ProjectApp {
                project_name: "example-app".to_owned(),
                name: "Example App".to_owned(),
                identifier: "dev.cefari.example-app".to_owned(),
                icon: None,
            },
            capabilities: vec![ProjectCapability::Tray {
                icon: Some("assets/tray-icon.png".to_owned()),
            }],
            frontend: FrontendConfig {
                dist: "frontend/dist".to_owned(),
                build_command: None,
                dev_command: None,
                dev_port: 5173,
            },
            daemon: DaemonConfig {
                entry: "daemon/main.ts".to_owned(),
            },
            package: PackageConfig {
                product_name: "Example App".to_owned(),
                version: "1.2.3".to_owned(),
            },
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-package-cef-{label}-{suffix}"))
    }
}
