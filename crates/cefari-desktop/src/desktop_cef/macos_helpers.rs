use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::paths::{
    CefRuntimePathConfig, MACOS_CEF_HELPER_SUFFIXES, MACOS_CEFARI_BUNDLE_IDENTIFIER,
    macos_helper_executable_name, macos_helper_executable_path,
};

pub(in crate::desktop_cef) fn prepare_macos_helper_apps(
    runtime_paths: &CefRuntimePathConfig,
    frameworks_dir: &Path,
) -> Result<()> {
    for suffix in MACOS_CEF_HELPER_SUFFIXES {
        let helper_name = macos_helper_executable_name(&runtime_paths.executable_path, suffix);
        prepare_macos_helper_app(runtime_paths, frameworks_dir, &helper_name)?;
    }

    Ok(())
}

fn prepare_macos_helper_app(
    runtime_paths: &CefRuntimePathConfig,
    frameworks_dir: &Path,
    helper_name: &str,
) -> Result<()> {
    let helper_exe = macos_helper_executable_path(frameworks_dir, helper_name);
    let helper_macos_dir = helper_exe
        .parent()
        .context("CEF helper executable path has no parent directory")?;
    let helper_contents_dir = helper_macos_dir
        .parent()
        .context("CEF helper app has no Contents directory")?;
    let helper_resources_dir = helper_contents_dir.join("Resources");

    fs::create_dir_all(helper_macos_dir).with_context(|| {
        format!(
            "failed to create CEF helper executable directory at {}",
            helper_macos_dir.display()
        )
    })?;
    fs::create_dir_all(&helper_resources_dir).with_context(|| {
        format!(
            "failed to create CEF helper resources directory at {}",
            helper_resources_dir.display()
        )
    })?;
    fs::write(
        helper_contents_dir.join("Info.plist"),
        macos_helper_info_plist(helper_name).as_bytes(),
    )
    .with_context(|| {
        format!(
            "failed to write CEF helper Info.plist under {}",
            helper_contents_dir.display()
        )
    })?;
    if runtime_paths.executable_path != helper_exe {
        replace_file_copy(&helper_exe, &runtime_paths.executable_path)?;
    }
    Ok(())
}

fn macos_helper_info_plist(helper_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>{helper_name}</string>
    <key>CFBundleExecutable</key>
    <string>{helper_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{MACOS_CEFARI_BUNDLE_IDENTIFIER}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{helper_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSUIElement</key>
    <string>1</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
"#
    )
}

pub(in crate::desktop_cef) fn create_clean_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove symlink at {}", path.display()))?;
        }
        Ok(metadata) if !metadata.is_dir() => {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove file at {}", path.display()))?;
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", path.display()));
        }
    }

    fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory at {}", path.display()))
}

pub(in crate::desktop_cef) fn replace_symlink(link: &Path, target: &Path) -> Result<()> {
    match fs::remove_file(link) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to remove {}", link.display()));
        }
    }
    std::os::unix::fs::symlink(target, link).with_context(|| {
        format!(
            "failed to create CEF loader framework symlink {} -> {}",
            link.display(),
            target.display()
        )
    })
}

fn replace_file_copy(destination: &Path, source: &Path) -> Result<()> {
    match fs::remove_file(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to remove {}", destination.display()));
        }
    }

    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy CEF helper executable {} -> {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}
