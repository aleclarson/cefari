use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result};
use clap::ValueEnum;

use crate::run_process;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SignPlatform {
    Macos,
    Windows,
    Linux,
}

impl SignPlatform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            _ => Self::Linux,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum UpdatePackageFormat {
    App,
    Appimage,
    Nsis,
    Wix,
}

impl UpdatePackageFormat {
    const fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Appimage => "appimage",
            Self::Nsis => "nsis",
            Self::Wix => "wix",
        }
    }
}

pub fn codesign(artifact: &Path, platform: SignPlatform, config: Option<&Path>) -> Result<()> {
    ensure_artifact(artifact)?;

    let mut command = Command::new("cargo-codesign");
    command.arg("codesign");
    push_config(&mut command, config);

    match platform {
        SignPlatform::Macos => {
            command.arg("macos");
            push_macos_artifact(&mut command, artifact)?;
            command.arg("--skip-notarize");
        }
        SignPlatform::Windows => {
            command.arg("windows");
        }
        SignPlatform::Linux => {
            command.arg("linux").arg("--archive").arg(artifact);
        }
    }

    run_process(&mut command, "cargo-codesign codesign")?;
    println!("signed artifact at {}", artifact.display());
    Ok(())
}

pub fn notarize(artifact: &Path, config: Option<&Path>) -> Result<()> {
    ensure_artifact(artifact)?;

    let mut command = Command::new("cargo-codesign");
    command.arg("codesign");
    push_config(&mut command, config);
    command.arg("macos");
    push_macos_artifact(&mut command, artifact)?;

    run_process(&mut command, "cargo-codesign notarize")?;
    println!("notarized artifact at {}", artifact.display());
    Ok(())
}

pub fn make_update(
    archive: &Path,
    url: &str,
    version: &str,
    target: &str,
    format: UpdatePackageFormat,
    key_env: &str,
    output_dir: &Path,
) -> Result<()> {
    ensure_artifact(archive)?;
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create update output directory at {}",
            output_dir.display()
        )
    })?;

    let signature_path = output_dir.join(format!("{}.sig", archive_stem(archive)?));
    let mut command = Command::new("cargo-codesign");
    command
        .arg("codesign")
        .arg("update")
        .arg("--archive")
        .arg(archive)
        .arg("--output")
        .arg(&signature_path)
        .arg("--key-env")
        .arg(key_env);

    run_process(&mut command, "cargo-codesign update")?;

    let signature = fs::read_to_string(&signature_path)
        .with_context(|| format!("failed to read signature at {}", signature_path.display()))?;
    let manifest = update_manifest(version, target, url, signature.trim(), format);
    let manifest_path = output_dir.join("update.json");
    fs::write(&manifest_path, manifest).with_context(|| {
        format!(
            "failed to write update manifest at {}",
            manifest_path.display()
        )
    })?;

    println!("generated update artifacts at {}", output_dir.display());
    Ok(())
}

pub fn default_update_target() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub fn default_update_format(target: &str) -> UpdatePackageFormat {
    if target.starts_with("macos-") || target.starts_with("darwin-") {
        UpdatePackageFormat::App
    } else if target.starts_with("windows-") {
        UpdatePackageFormat::Nsis
    } else {
        UpdatePackageFormat::Appimage
    }
}

fn push_config(command: &mut Command, config: Option<&Path>) {
    if let Some(config) = config {
        command.arg("--config").arg(config);
    }
}

fn push_macos_artifact(command: &mut Command, artifact: &Path) -> Result<()> {
    match artifact
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("app") => {
            command.arg("--app").arg(artifact);
            Ok(())
        }
        Some("dmg") => {
            command.arg("--dmg").arg(artifact);
            Ok(())
        }
        _ => anyhow::bail!(
            "macOS signing requires a .app bundle or .dmg artifact: {}",
            artifact.display()
        ),
    }
}

fn ensure_artifact(path: &Path) -> Result<()> {
    if path.exists() {
        Ok(())
    } else {
        anyhow::bail!("artifact does not exist: {}", path.display())
    }
}

fn archive_stem(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace('/', "-"))
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow::anyhow!("archive path has no file name: {}", path.display()))
}

fn update_manifest(
    version: &str,
    target: &str,
    url: &str,
    signature: &str,
    format: UpdatePackageFormat,
) -> String {
    format!(
        r#"{{
  "version": "{}",
  "platforms": {{
    "{}": {{
      "format": "{}",
      "signature": "{}",
      "url": "{}"
    }}
  }}
}}
"#,
        json_escape(version),
        json_escape(target),
        format.as_str(),
        json_escape(signature),
        json_escape(url)
    )
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{
        UpdatePackageFormat, default_update_format, default_update_target, update_manifest,
    };

    #[test]
    fn default_update_target_includes_os_and_arch() {
        let target = default_update_target();
        assert!(target.contains(std::env::consts::OS));
        assert!(target.contains(std::env::consts::ARCH));
    }

    #[test]
    fn default_update_format_follows_target_platform() {
        assert_eq!(
            default_update_format("macos-aarch64"),
            UpdatePackageFormat::App
        );
        assert_eq!(
            default_update_format("darwin-aarch64"),
            UpdatePackageFormat::App
        );
        assert_eq!(
            default_update_format("windows-x86_64"),
            UpdatePackageFormat::Nsis
        );
        assert_eq!(
            default_update_format("linux-x86_64"),
            UpdatePackageFormat::Appimage
        );
    }

    #[test]
    fn update_manifest_escapes_json_fields() {
        let manifest = update_manifest(
            "1.0.0",
            "darwin-aarch64",
            "https://e.test/a\"b",
            "sig",
            UpdatePackageFormat::App,
        );
        assert!(manifest.contains(r#""version": "1.0.0""#));
        assert!(manifest.contains(r#""url": "https://e.test/a\"b""#));
        assert!(manifest.contains(r#""format": "app""#));
    }
}
