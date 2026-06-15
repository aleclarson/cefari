use std::path::{Path, PathBuf};

use cefari_core::{PackageFormat, RuntimePaths, packaged_resources_dir};

const CEF_RESOURCES_DIR_ENV: &str = "CEFARI_CEF_RESOURCES_DIR";

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) const MACOS_CEFARI_BUNDLE_IDENTIFIER: &str = "dev.cefari.app";
#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) const MACOS_CEF_HELPER_SUFFIXES: &[&str] = &[
    "Helper (GPU)",
    "Helper (Renderer)",
    "Helper (Plugin)",
    "Helper (Alerts)",
    "Helper",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::desktop_cef) struct CefRuntimePathConfig {
    pub(in crate::desktop_cef) cache_path: PathBuf,
    pub(in crate::desktop_cef) root_cache_path: PathBuf,
    pub(in crate::desktop_cef) log_file: PathBuf,
    pub(in crate::desktop_cef) executable_path: PathBuf,
    pub(in crate::desktop_cef) browser_subprocess_path: Option<PathBuf>,
    pub(in crate::desktop_cef) main_bundle_path: Option<PathBuf>,
    pub(in crate::desktop_cef) resources_dir_path: Option<PathBuf>,
    pub(in crate::desktop_cef) locales_dir_path: Option<PathBuf>,
    pub(in crate::desktop_cef) framework_dir_path: Option<PathBuf>,
}

pub(in crate::desktop_cef) fn resolve_cef_runtime_paths(
    paths: &RuntimePaths,
) -> CefRuntimePathConfig {
    let executable_path =
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cefari-desktop"));
    cef_runtime_path_config(paths, cef_resource_dir_candidates(paths), &executable_path)
}

fn cef_runtime_path_config(
    paths: &RuntimePaths,
    resource_candidates: Vec<PathBuf>,
    executable_path: &Path,
) -> CefRuntimePathConfig {
    let resources_dir_path = resource_candidates
        .into_iter()
        .find(|candidate| candidate.join("archive.json").is_file());
    let locales_dir_path = resources_dir_path
        .as_ref()
        .map(|resources_dir| resources_dir.join("locales"))
        .filter(|locales_dir| locales_dir.is_dir());
    let framework_dir_path = resources_dir_path
        .as_ref()
        .and_then(|resources_dir| cef_framework_dir(resources_dir));
    let root_cache_path = paths.cache_dir.join("cef");
    let browser_subprocess_path = cef_browser_subprocess_path(&root_cache_path, executable_path);
    let main_bundle_path = cef_main_bundle_path(executable_path);

    CefRuntimePathConfig {
        cache_path: root_cache_path.join("profile"),
        root_cache_path,
        log_file: paths.log_dir.join("cef.log"),
        executable_path: executable_path.to_path_buf(),
        browser_subprocess_path,
        main_bundle_path,
        resources_dir_path,
        locales_dir_path,
        framework_dir_path,
    }
}

#[cfg(target_os = "macos")]
fn cef_browser_subprocess_path(root_cache_path: &Path, executable_path: &Path) -> Option<PathBuf> {
    if macos_app_contents_dir(executable_path).is_some() {
        return None;
    }

    Some(macos_helper_executable_path(
        &macos_frameworks_dir(root_cache_path, executable_path),
        &macos_helper_executable_name(executable_path, "Helper"),
    ))
}

#[cfg(not(target_os = "macos"))]
fn cef_browser_subprocess_path(_root_cache_path: &Path, executable_path: &Path) -> Option<PathBuf> {
    Some(executable_path.to_path_buf())
}

#[cfg(target_os = "macos")]
fn cef_main_bundle_path(executable_path: &Path) -> Option<PathBuf> {
    macos_host_app_contents_dir(executable_path)
        .or_else(|| macos_app_contents_dir(executable_path))
        .and_then(|contents_dir| contents_dir.parent().map(Path::to_path_buf))
}

#[cfg(not(target_os = "macos"))]
fn cef_main_bundle_path(_executable_path: &Path) -> Option<PathBuf> {
    None
}

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) fn macos_frameworks_dir(
    root_cache_path: &Path,
    executable_path: &Path,
) -> PathBuf {
    macos_host_app_contents_dir(executable_path).map_or_else(
        || root_cache_path.join("loader-layout").join("Frameworks"),
        |contents_dir| contents_dir.join("Frameworks"),
    )
}

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) fn macos_loader_executable(
    root_cache_path: &Path,
    executable_path: &Path,
) -> PathBuf {
    if macos_app_contents_dir(executable_path).is_some() {
        return executable_path.to_path_buf();
    }

    root_cache_path
        .join("loader-layout")
        .join("MacOS")
        .join("cefari-desktop")
}

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) fn macos_is_helper_executable(executable_path: &Path) -> bool {
    macos_helper_host_contents_dir(executable_path).is_some()
}

#[cfg(target_os = "macos")]
fn macos_host_app_contents_dir(executable_path: &Path) -> Option<PathBuf> {
    macos_helper_host_contents_dir(executable_path)
        .or_else(|| macos_app_contents_dir(executable_path))
}

#[cfg(target_os = "macos")]
fn macos_helper_host_contents_dir(executable_path: &Path) -> Option<PathBuf> {
    let helper_contents_dir = macos_app_contents_dir(executable_path)?;
    let helper_app_dir = helper_contents_dir.parent()?;
    let frameworks_dir = helper_app_dir.parent()?;
    if frameworks_dir.file_name()? != "Frameworks" {
        return None;
    }

    let host_contents_dir = frameworks_dir.parent()?;
    (host_contents_dir.file_name()? == "Contents").then_some(host_contents_dir.to_path_buf())
}

#[cfg(target_os = "macos")]
fn macos_app_contents_dir(executable_path: &Path) -> Option<PathBuf> {
    let macos_dir = executable_path.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }

    let contents_dir = macos_dir.parent()?;
    if contents_dir.file_name()? != "Contents" {
        return None;
    }

    let app_dir = contents_dir.parent()?;
    (app_dir.extension()? == "app").then_some(contents_dir.to_path_buf())
}

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) fn macos_helper_executable_name(
    executable_path: &Path,
    suffix: &str,
) -> String {
    let app_executable_name = executable_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cefari-desktop");
    format!("{app_executable_name} {suffix}")
}

#[cfg(target_os = "macos")]
pub(in crate::desktop_cef) fn macos_helper_executable_path(
    frameworks_dir: &Path,
    helper_name: &str,
) -> PathBuf {
    frameworks_dir
        .join(format!("{helper_name}.app"))
        .join("Contents")
        .join("MacOS")
        .join(helper_name)
}

fn cef_resource_dir_candidates(paths: &RuntimePaths) -> Vec<PathBuf> {
    let mut candidates = std::env::var_os(CEF_RESOURCES_DIR_ENV)
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();

    candidates.extend(platform_package_formats().iter().filter_map(|format| {
        packaged_resources_dir(*format)
            .ok()
            .map(|resources_dir| resources_dir.join("cef"))
    }));
    candidates.push(paths.resource_dir.join("cef"));
    if let Some(cef_dir) = cef::sys::get_cef_dir() {
        candidates.push(cef_dir);
    }
    candidates
}

fn cef_framework_dir(resources_dir: &Path) -> Option<PathBuf> {
    let framework_dir = resources_dir.join("Chromium Embedded Framework.framework");
    framework_dir.is_dir().then_some(framework_dir)
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use cefari_core::RuntimePaths;

    use super::cef_runtime_path_config;

    #[test]
    fn derives_runtime_cache_log_and_subprocess_paths() {
        let root = temp_dir("runtime-paths");
        let paths = test_paths(&root);
        let subprocess = root.join("bin/cefari-desktop");

        let config = cef_runtime_path_config(&paths, Vec::new(), &subprocess);

        assert_eq!(config.cache_path, root.join("cache/cef/profile"));
        assert_eq!(config.root_cache_path, root.join("cache/cef"));
        assert_eq!(config.log_file, root.join("data/logs/cef.log"));
        assert_eq!(config.executable_path, subprocess);
        #[cfg(target_os = "macos")]
        assert_eq!(
            config.browser_subprocess_path,
            Some(root.join(
                "cache/cef/loader-layout/Frameworks/cefari-desktop Helper.app/Contents/MacOS/cefari-desktop Helper"
            ))
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(config.browser_subprocess_path, Some(subprocess));
        assert!(config.main_bundle_path.is_none());
        assert!(config.resources_dir_path.is_none());
        assert!(config.locales_dir_path.is_none());
        assert!(config.framework_dir_path.is_none());

        if root.exists() {
            fs::remove_dir_all(root).expect("temp dir should be removable");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn leaves_subprocess_path_unset_for_macos_app_bundle_launch() {
        let root = temp_dir("runtime-app-paths");
        let paths = test_paths(&root);
        let executable =
            root.join("cache/dev-app/cefari-desktop.app/Contents/MacOS/cefari-desktop");

        let config = cef_runtime_path_config(&paths, Vec::new(), &executable);

        assert_eq!(config.executable_path, executable);
        assert_eq!(config.browser_subprocess_path, None);
        assert_eq!(
            config.main_bundle_path,
            Some(root.join("cache/dev-app/cefari-desktop.app"))
        );

        if root.exists() {
            fs::remove_dir_all(root).expect("temp dir should be removable");
        }
    }

    #[test]
    fn selects_first_valid_cef_resource_candidate() {
        let root = temp_dir("resource-candidates");
        let missing = root.join("missing-cef");
        let resources = root.join("resources-cef");
        fs::create_dir_all(resources.join("locales")).expect("locales dir should exist");
        fs::create_dir_all(resources.join("Chromium Embedded Framework.framework"))
            .expect("framework dir should exist");
        fs::write(resources.join("archive.json"), "{}").expect("archive metadata should exist");
        let paths = test_paths(&root);

        let config = cef_runtime_path_config(
            &paths,
            vec![missing, resources.clone()],
            &root.join("cefari-desktop"),
        );

        assert_eq!(config.resources_dir_path, Some(resources.clone()));
        assert_eq!(config.locales_dir_path, Some(resources.join("locales")));
        assert_eq!(
            config.framework_dir_path,
            Some(resources.join("Chromium Embedded Framework.framework"))
        );

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn resolves_runtime_resource_dir_as_fallback_candidate() {
        let root = temp_dir("runtime-fallback");
        let paths = test_paths(&root);
        let cef_dir = paths.resource_dir.join("cef");
        fs::create_dir_all(&cef_dir).expect("CEF resource dir should exist");
        fs::write(cef_dir.join("archive.json"), "{}").expect("archive metadata should exist");

        let desktop = root.join("desktop");
        let config = cef_runtime_path_config(&paths, vec![cef_dir.clone()], &desktop);

        assert_eq!(config.resources_dir_path, Some(cef_dir));

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    fn test_paths(root: &Path) -> RuntimePaths {
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
        std::env::temp_dir().join(format!("cefari-desktop-cef-{label}-{suffix}"))
    }
}
