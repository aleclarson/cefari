use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use cefari_core::{PackageFormat, RuntimePaths, packaged_resources_dir, resolve_resource};

pub const UI_ENTRY_RESOURCE: &str = "frontend/index.html";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellUi {
    pub entry_path: PathBuf,
    pub state: ShellUiState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellUiState {
    PackagedResource,
    RuntimeResourceFallback,
    DiagnosticFallback { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceCandidate {
    state: ShellUiState,
    dir: PathBuf,
}

impl ShellUi {
    pub fn load(paths: &RuntimePaths) -> Result<Self> {
        load_from_candidates(paths, candidate_resource_dirs(paths))
    }

    pub fn is_diagnostic(&self) -> bool {
        matches!(self.state, ShellUiState::DiagnosticFallback { .. })
    }
}

fn load_from_candidates(
    paths: &RuntimePaths,
    candidates: Vec<ResourceCandidate>,
) -> Result<ShellUi> {
    let mut misses = Vec::new();

    for candidate in candidates {
        match resolve_resource(&candidate.dir, UI_ENTRY_RESOURCE) {
            Ok(entry_path) => {
                return Ok(ShellUi {
                    entry_path,
                    state: candidate.state,
                });
            }
            Err(error) => {
                misses.push(format!("{}: {error}", candidate.dir.display()));
            }
        }
    }

    let reason = format!(
        "missing {UI_ENTRY_RESOURCE}; checked {}",
        if misses.is_empty() {
            "no resource directories".to_owned()
        } else {
            misses.join("; ")
        }
    );
    let entry_path = write_diagnostic_view(paths, &reason)?;

    Ok(ShellUi {
        entry_path,
        state: ShellUiState::DiagnosticFallback { reason },
    })
}

fn candidate_resource_dirs(paths: &RuntimePaths) -> Vec<ResourceCandidate> {
    let mut candidates = platform_package_formats()
        .iter()
        .filter_map(|format| {
            packaged_resources_dir(*format)
                .ok()
                .map(|dir| ResourceCandidate {
                    state: ShellUiState::PackagedResource,
                    dir,
                })
        })
        .collect::<Vec<_>>();

    candidates.push(ResourceCandidate {
        state: ShellUiState::RuntimeResourceFallback,
        dir: paths.resource_dir.clone(),
    });
    candidates
}

#[cfg(target_os = "macos")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::App, PackageFormat::Dmg]
}

#[cfg(target_os = "windows")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::Nsis, PackageFormat::Wix]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[
        PackageFormat::Deb,
        PackageFormat::AppImage,
        PackageFormat::Pacman,
    ]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[]
}

fn write_diagnostic_view(paths: &RuntimePaths, reason: &str) -> Result<PathBuf> {
    let diagnostic_dir = paths.cache_dir.join("diagnostics");
    fs::create_dir_all(&diagnostic_dir).with_context(|| {
        format!(
            "failed to create diagnostic UI directory at {}",
            diagnostic_dir.display()
        )
    })?;

    let diagnostic_path = diagnostic_dir.join("missing-ui.html");
    fs::write(&diagnostic_path, diagnostic_view_html(reason)).with_context(|| {
        format!(
            "failed to write diagnostic UI at {}",
            diagnostic_path.display()
        )
    })?;
    Ok(diagnostic_path)
}

fn diagnostic_view_html(reason: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Cefari Diagnostic</title>
<body>
<h1>Cefari UI resources are missing</h1>
<p>{}</p>
</body>
</html>
"#,
        escape_html(reason)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use cefari_core::RuntimePaths;

    use super::{
        ResourceCandidate, ShellUiState, UI_ENTRY_RESOURCE, diagnostic_view_html,
        load_from_candidates,
    };

    #[test]
    fn loads_existing_frontend_entry_from_first_matching_candidate() {
        let root = temp_dir("resource-hit");
        let resource_dir = root.join("resources");
        fs::create_dir_all(resource_dir.join("frontend")).expect("resource dir should exist");
        fs::write(resource_dir.join(UI_ENTRY_RESOURCE), "<!doctype html>")
            .expect("resource should be written");
        let paths = test_paths(&root);

        let ui = load_from_candidates(
            &paths,
            vec![ResourceCandidate {
                state: ShellUiState::RuntimeResourceFallback,
                dir: resource_dir.clone(),
            }],
        )
        .expect("UI should load");

        assert_eq!(ui.entry_path, resource_dir.join(UI_ENTRY_RESOURCE));
        assert_eq!(ui.state, ShellUiState::RuntimeResourceFallback);

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn writes_diagnostic_view_when_frontend_entry_is_missing() {
        let root = temp_dir("resource-miss");
        let paths = test_paths(&root);

        let ui = load_from_candidates(&paths, Vec::new()).expect("diagnostic UI should be written");

        assert!(ui.is_diagnostic());
        assert!(ui.entry_path.ends_with("missing-ui.html"));
        assert!(ui.entry_path.exists());

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn diagnostic_view_escapes_reason_text() {
        let html = diagnostic_view_html("<missing & broken>");

        assert!(html.contains("&lt;missing &amp; broken&gt;"));
    }

    fn test_paths(root: &std::path::Path) -> RuntimePaths {
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config/cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("data/logs"),
            resource_dir: root.join("data/resources"),
            update_dir: root.join("data/updates"),
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-desktop-{label}-{suffix}"))
    }
}
