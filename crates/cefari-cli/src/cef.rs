use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::project::ProjectConfig;

const CEF_VERSION: &str = "148.4.0";

pub struct PreparedCef {
    pub resources_dir: std::path::PathBuf,
}

pub fn prepare_cef(project_dir: &Path) -> Result<PreparedCef> {
    let cef_dir = ProjectConfig::build_dir(project_dir).join("cef");
    let resources_dir = cef_dir.join("resources");
    fs::create_dir_all(&resources_dir).with_context(|| {
        format!(
            "failed to create CEF resources directory at {}",
            resources_dir.display()
        )
    })?;

    let manifest = CefPreparationManifest {
        version: CEF_VERSION,
        target_os: std::env::consts::OS,
        target_arch: std::env::consts::ARCH,
        source: "pending-download",
        resources_dir: &normalize(&resources_dir),
    };

    fs::write(cef_dir.join("manifest.json"), manifest.to_json()).with_context(|| {
        format!(
            "failed to write CEF preparation manifest at {}",
            cef_dir.join("manifest.json").display()
        )
    })?;

    Ok(PreparedCef { resources_dir })
}

pub fn prepared_resources_dir(project_dir: &Path) -> Result<std::path::PathBuf> {
    let resources_dir = ProjectConfig::build_dir(project_dir).join("cef/resources");
    if resources_dir.exists() {
        Ok(resources_dir)
    } else {
        anyhow::bail!(
            "missing prepared CEF resources at {}; run cefari build first",
            resources_dir.display()
        )
    }
}

struct CefPreparationManifest<'a> {
    version: &'a str,
    target_os: &'a str,
    target_arch: &'a str,
    source: &'a str,
    resources_dir: &'a str,
}

impl CefPreparationManifest<'_> {
    fn to_json(&self) -> String {
        format!(
            r#"{{
  "version": "{}",
  "target_os": "{}",
  "target_arch": "{}",
  "source": "{}",
  "resources_dir": "{}"
}}
"#,
            self.version, self.target_os, self.target_arch, self.source, self.resources_dir
        )
    }
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::prepare_cef;

    #[test]
    fn prepares_cef_manifest_and_resources_dir() {
        let root =
            std::env::temp_dir().join(format!("cefari-cef-prep-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        let prepared = prepare_cef(&root).expect("CEF preparation should succeed");

        assert!(prepared.resources_dir.exists());
        let manifest =
            std::fs::read_to_string(root.join("build/cef/manifest.json")).expect("manifest");
        assert!(manifest.contains(r#""version": "148.4.0""#));
        assert!(manifest.contains(r#""source": "pending-download""#));

        std::fs::remove_dir_all(root).expect("temp dir should be removable");
    }
}
