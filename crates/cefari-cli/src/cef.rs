use std::{fs, path::Path, time::Duration};

use anyhow::{Context, Result};
use download_cef::{CefFile, CefIndex, DEFAULT_TARGET};
use serde::Serialize;

use crate::project::ProjectConfig;

const CEF_VERSION: &str = "148.4.0";
const CEF_ARCHIVE_VERSION: &str = "148.0.10";

pub struct PreparedCef {
    pub resources_dir: std::path::PathBuf,
    pub cache_dir: std::path::PathBuf,
    pub archive_json: std::path::PathBuf,
}

pub fn prepare_cef(project_dir: &Path) -> Result<PreparedCef> {
    let resources_override =
        std::env::var_os("CEFARI_CEF_RESOURCES_DIR").map(std::path::PathBuf::from);
    let archive_override = std::env::var_os("CEFARI_CEF_ARCHIVE").map(std::path::PathBuf::from);
    prepare_cef_with_overrides(
        project_dir,
        resources_override.as_deref(),
        archive_override.as_deref(),
    )
}

fn prepare_cef_with_overrides(
    project_dir: &Path,
    resources_override: Option<&Path>,
    archive_override: Option<&Path>,
) -> Result<PreparedCef> {
    let cef_dir = ProjectConfig::build_dir(project_dir).join("cef");
    let resources_dir = cef_dir.join("resources");
    let cache_dir = ProjectConfig::build_dir(project_dir).join("cef-cache");
    populate_cef_resources(
        &resources_dir,
        &cache_dir,
        resources_override,
        archive_override,
    )?;

    let archive_json = resources_dir.join("archive.json");
    let archive = read_archive_file(&archive_json)?;
    fs::create_dir_all(&cef_dir).with_context(|| {
        format!(
            "failed to create CEF preparation directory at {}",
            cef_dir.display()
        )
    })?;

    let manifest = CefPreparationManifest {
        version: CEF_VERSION,
        archive_version: CEF_ARCHIVE_VERSION,
        target_os: std::env::consts::OS,
        target_arch: std::env::consts::ARCH,
        target: DEFAULT_TARGET,
        source: archive.name.as_str(),
        sha1: archive.sha1.as_str(),
        cache_dir: &normalize(&cache_dir),
        resources_dir: &normalize(&resources_dir),
    };

    let manifest_json = manifest.to_json()?;
    fs::write(cef_dir.join("manifest.json"), manifest_json).with_context(|| {
        format!(
            "failed to write CEF preparation manifest at {}",
            cef_dir.join("manifest.json").display()
        )
    })?;

    Ok(PreparedCef {
        resources_dir,
        cache_dir,
        archive_json,
    })
}

pub fn prepared_resources_dir(project_dir: &Path) -> Result<std::path::PathBuf> {
    let resources_dir = ProjectConfig::build_dir(project_dir).join("cef/resources");
    if resources_dir.join("archive.json").exists() {
        Ok(resources_dir)
    } else {
        anyhow::bail!(
            "missing downloaded CEF resources at {}; run cefari build first",
            resources_dir.display()
        )
    }
}

fn populate_cef_resources(
    resources_dir: &Path,
    cache_dir: &Path,
    resources_override: Option<&Path>,
    archive_override: Option<&Path>,
) -> Result<()> {
    if resources_dir.join("archive.json").exists() {
        download_cef::check_archive_json(CEF_ARCHIVE_VERSION, &normalize(resources_dir))
            .context("failed to verify cached CEF resources")?;
        return Ok(());
    }

    if let Some(fixture_dir) = resources_override {
        copy_dir_recursive(fixture_dir, resources_dir).with_context(|| {
            format!(
                "failed to copy CEF resources fixture from {} to {}",
                fixture_dir.display(),
                resources_dir.display()
            )
        })?;
        download_cef::check_archive_json(CEF_ARCHIVE_VERSION, &normalize(resources_dir))
            .context("failed to verify CEF resources fixture")?;
        return Ok(());
    }

    let extracted_dir = if let Some(archive) = archive_override {
        fs::create_dir_all(cache_dir)
            .with_context(|| format!("failed to create CEF cache at {}", cache_dir.display()))?;
        let archive_file = CefFile::try_from(archive)
            .with_context(|| format!("failed to inspect CEF archive at {}", archive.display()))?;
        let extracted =
            download_cef::extract_target_archive(DEFAULT_TARGET, archive, cache_dir, true)
                .with_context(|| format!("failed to extract CEF archive {}", archive.display()))?;
        archive_file
            .write_archive_json(&extracted)
            .context("failed to write CEF archive metadata")?;
        extracted
    } else {
        let mirror_url = std::env::var("CEF_DOWNLOAD_URL")
            .unwrap_or_else(|_| download_cef::default_download_url());
        let index = CefIndex::download_from(&mirror_url).context("failed to download CEF index")?;
        let platform = index
            .platform(DEFAULT_TARGET)
            .with_context(|| format!("failed to resolve CEF platform for {DEFAULT_TARGET}"))?;
        let version = platform
            .version(CEF_ARCHIVE_VERSION)
            .with_context(|| format!("failed to resolve CEF version {CEF_ARCHIVE_VERSION}"))?;

        let archive = version
            .download_archive_with_retry_from(
                &mirror_url,
                cache_dir,
                true,
                Duration::from_secs(15),
                3,
            )
            .context("failed to download CEF archive")?;
        let extracted =
            download_cef::extract_target_archive(DEFAULT_TARGET, &archive, cache_dir, true)
                .with_context(|| format!("failed to extract CEF archive {}", archive.display()))?;
        version
            .minimal()
            .context("failed to identify downloaded CEF archive")?
            .write_archive_json(&extracted)
            .context("failed to write CEF archive metadata")?;
        extracted
    };

    replace_resources_dir(&extracted_dir, resources_dir)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination).with_context(|| {
            format!(
                "failed to remove old CEF resources at {}",
                destination.display()
            )
        })?;
    }
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "failed to create CEF resources at {}",
            destination.display()
        )
    })?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read CEF resources at {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy CEF resource from {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn replace_resources_dir(extracted_dir: &Path, resources_dir: &Path) -> Result<()> {
    if extracted_dir == resources_dir {
        return Ok(());
    }

    if resources_dir.exists() {
        fs::remove_dir_all(resources_dir).with_context(|| {
            format!(
                "failed to remove old CEF resources at {}",
                resources_dir.display()
            )
        })?;
    }
    if let Some(parent) = resources_dir.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create CEF resources parent at {}",
                parent.display()
            )
        })?;
    }
    fs::rename(extracted_dir, resources_dir).with_context(|| {
        format!(
            "failed to move extracted CEF resources from {} to {}",
            extracted_dir.display(),
            resources_dir.display()
        )
    })
}

fn read_archive_file(path: &Path) -> Result<CefFile> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open CEF archive metadata at {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse CEF archive metadata at {}", path.display()))
}

#[derive(Serialize)]
struct CefPreparationManifest<'a> {
    version: &'a str,
    archive_version: &'a str,
    target_os: &'a str,
    target_arch: &'a str,
    target: &'a str,
    source: &'a str,
    sha1: &'a str,
    cache_dir: &'a str,
    resources_dir: &'a str,
}

impl CefPreparationManifest<'_> {
    fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("failed to encode CEF preparation manifest")
    }
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{CefPreparationManifest, prepare_cef_with_overrides};

    #[test]
    fn prepares_cef_manifest_and_resources_dir() {
        let root =
            std::env::temp_dir().join(format!("cefari-cef-prep-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let fixture = root.join("fixture");
        fs::create_dir_all(&fixture).expect("fixture should be created");
        fs::write(
            fixture.join("archive.json"),
            r#"{
  "type": "minimal",
  "name": "cef_binary_148.0.10+gfixture+chromium-148.0.0_macosarm64_minimal.tar.bz2",
  "sha1": "fixture-sha1"
}"#,
        )
        .expect("archive metadata should be written");
        fs::write(fixture.join("libcef.dylib"), "fixture").expect("fixture resource");

        let prepared = prepare_cef_with_overrides(&root, Some(&fixture), None)
            .expect("CEF preparation should succeed");

        assert!(prepared.resources_dir.exists());
        assert!(prepared.archive_json.exists());
        assert!(prepared.resources_dir.join("libcef.dylib").exists());
        let manifest =
            std::fs::read_to_string(root.join("build/cef/manifest.json")).expect("manifest");
        assert!(manifest.contains(r#""version": "148.4.0""#));
        assert!(manifest.contains(r#""archive_version": "148.0.10""#));
        assert!(manifest.contains(r#""source": "cef_binary_148.0.10"#));
        assert!(manifest.contains(r#""sha1": "fixture-sha1""#));

        std::fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn cef_preparation_manifest_escapes_json_strings() {
        let manifest = CefPreparationManifest {
            version: "148.4.0",
            archive_version: "148.0.10",
            target_os: "macos",
            target_arch: "aarch64",
            target: "macosarm64",
            source: "https://example.test/quoted\"archive.tar.bz2",
            sha1: "fixture\\sha\nnext",
            cache_dir: "/tmp/cache",
            resources_dir: "/tmp/resources",
        };

        let json = manifest.to_json().expect("manifest should serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("manifest should be valid JSON");

        assert_eq!(
            value["source"],
            "https://example.test/quoted\"archive.tar.bz2"
        );
        assert_eq!(value["sha1"], "fixture\\sha\nnext");
    }
}
