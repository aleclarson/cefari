use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};
use cefari_core::{PackageFormat, RuntimePaths, packaged_resources_dir, resolve_resource};

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
pub const CEFARI_APP_SCHEME: &str = "cefari";
#[cfg_attr(not(feature = "cef"), allow(dead_code))]
pub const CEFARI_APP_HOST: &str = "app";
pub const CEFARI_APP_ORIGIN: &str = "cefari://app";
pub const UI_ENTRY_RESOURCE: &str = "frontend/index.html";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellUi {
    pub entry_path: PathBuf,
    app_resource_dir: PathBuf,
    app_entry_url: String,
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

    pub fn url(&self) -> String {
        std::env::var("CEFARI_FRONTEND_URL").unwrap_or_else(|_| self.app_entry_url.clone())
    }

    pub fn app_resource_dir(&self) -> &Path {
        &self.app_resource_dir
    }
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSchemeResource {
    pub path: PathBuf,
    pub mime_type: &'static str,
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppSchemeResourceError {
    InvalidUrl,
    UnsafePath,
    Missing { path: PathBuf },
}

impl std::fmt::Display for AppSchemeResourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(formatter, "invalid app-scheme URL"),
            Self::UnsafePath => write!(formatter, "unsafe app-scheme resource path"),
            Self::Missing { path } => {
                write!(formatter, "missing app-scheme resource {}", path.display())
            }
        }
    }
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
#[allow(dead_code)]
pub fn resolve_app_scheme_resource(resource_dir: &Path, url: &str) -> Option<AppSchemeResource> {
    diagnose_app_scheme_resource(resource_dir, url).ok()
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
pub fn diagnose_app_scheme_resource(
    resource_dir: &Path,
    url: &str,
) -> Result<AppSchemeResource, AppSchemeResourceError> {
    let resource_path = app_scheme_resource_path(url).ok_or(AppSchemeResourceError::InvalidUrl)?;
    let path = resolve_safe_resource_path(resource_dir, &resource_path)
        .ok_or(AppSchemeResourceError::UnsafePath)?;

    if path.is_file() {
        Ok(AppSchemeResource {
            mime_type: mime_type_for_path(&path),
            path,
        })
    } else {
        Err(AppSchemeResourceError::Missing { path })
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
                let app_resource_dir = entry_path
                    .parent()
                    .context("UI entry resource has no parent directory")?
                    .to_path_buf();
                let app_entry_url = app_url_for_entry(&entry_path)?;
                return Ok(ShellUi {
                    entry_path,
                    app_resource_dir,
                    app_entry_url,
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
        app_resource_dir: entry_path
            .parent()
            .context("diagnostic UI has no parent directory")?
            .to_path_buf(),
        app_entry_url: app_url_for_entry(&entry_path)?,
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

fn app_url_for_entry(entry_path: &Path) -> Result<String> {
    let file_name = entry_path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .context("UI entry file name is not valid UTF-8")?;
    Ok(format!("{CEFARI_APP_ORIGIN}/{file_name}"))
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
fn app_scheme_resource_path(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case(CEFARI_APP_SCHEME) {
        return None;
    }

    let authority_end = rest
        .find(|character| matches!(character, '/' | '?' | '#'))
        .unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if !authority.eq_ignore_ascii_case(CEFARI_APP_HOST) {
        return None;
    }

    let after_authority = &rest[authority_end..];
    let path_end = after_authority
        .find(|character| matches!(character, '?' | '#'))
        .unwrap_or(after_authority.len());
    let resource_path = after_authority[..path_end].trim_start_matches('/');
    if resource_path.is_empty() {
        Some("index.html".to_owned())
    } else {
        Some(resource_path.to_owned())
    }
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
fn resolve_safe_resource_path(resource_dir: &Path, resource_path: &str) -> Option<PathBuf> {
    let mut path = resource_dir.to_path_buf();
    for component in Path::new(resource_path).components() {
        match component {
            Component::Normal(segment) => path.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(path)
}

#[cfg_attr(not(feature = "cef"), allow(dead_code))]
fn mime_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("css") => "text/css",
        Some("html") | Some("htm") => "text/html",
        Some("js") | Some("mjs") => "text/javascript",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
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
        CEFARI_APP_ORIGIN, ResourceCandidate, ShellUiState, UI_ENTRY_RESOURCE,
        diagnose_app_scheme_resource, diagnostic_view_html, load_from_candidates,
        resolve_app_scheme_resource,
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
        assert_eq!(ui.app_resource_dir(), resource_dir.join("frontend"));
        assert_eq!(ui.url(), format!("{CEFARI_APP_ORIGIN}/index.html"));
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
        assert_eq!(ui.app_resource_dir(), root.join("cache/diagnostics"));
        assert_eq!(ui.url(), format!("{CEFARI_APP_ORIGIN}/missing-ui.html"));
        assert!(ui.entry_path.exists());

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn diagnostic_view_escapes_reason_text() {
        let html = diagnostic_view_html("<missing & broken>");

        assert!(html.contains("&lt;missing &amp; broken&gt;"));
    }

    #[test]
    fn app_scheme_resolves_files_under_resource_dir() {
        let root = temp_dir("app-scheme-hit");
        fs::create_dir_all(root.join("assets")).expect("asset dir should exist");
        fs::write(root.join("index.html"), "<!doctype html>").expect("index should be written");
        fs::write(root.join("assets/app.js"), "console.log('ok')")
            .expect("script should be written");

        let index = resolve_app_scheme_resource(&root, "cefari://app/")
            .expect("default index should resolve");
        let script = resolve_app_scheme_resource(&root, "cefari://app/assets/app.js?cache=1")
            .expect("script should resolve");

        assert_eq!(index.path, root.join("index.html"));
        assert_eq!(index.mime_type, "text/html");
        assert_eq!(script.path, root.join("assets/app.js"));
        assert_eq!(script.mime_type, "text/javascript");

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn app_scheme_rejects_non_app_urls_and_traversal() {
        let root = temp_dir("app-scheme-reject");
        fs::create_dir_all(&root).expect("root should exist");

        assert!(resolve_app_scheme_resource(&root, "file:///tmp/index.html").is_none());
        assert!(resolve_app_scheme_resource(&root, "cefari://other/index.html").is_none());
        assert!(resolve_app_scheme_resource(&root, "cefari://app/../secret.txt").is_none());
        assert!(resolve_app_scheme_resource(&root, "cefari://app/missing.css").is_none());

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn app_scheme_diagnostics_classify_resource_failures() {
        let root = temp_dir("app-scheme-diagnostics");
        fs::create_dir_all(&root).expect("root should exist");

        let invalid = diagnose_app_scheme_resource(&root, "file:///tmp/index.html")
            .expect_err("invalid URL should be diagnosed");
        let unsafe_path = diagnose_app_scheme_resource(&root, "cefari://app/../secret.txt")
            .expect_err("unsafe path should be diagnosed");
        let missing = diagnose_app_scheme_resource(&root, "cefari://app/missing.css")
            .expect_err("missing file should be diagnosed");

        assert_eq!(invalid.to_string(), "invalid app-scheme URL");
        assert_eq!(unsafe_path.to_string(), "unsafe app-scheme resource path");
        assert!(missing.to_string().contains("missing.css"));

        fs::remove_dir_all(root).expect("temp dir should be removable");
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
