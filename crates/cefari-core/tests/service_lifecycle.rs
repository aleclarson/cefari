use std::{
    env,
    ffi::OsString,
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use std::{fs, path::PathBuf};

use cefari_core::{
    CefariServiceSpec, install_service, service_manager, service_status, start_service,
    stop_service, uninstall_service,
};
use service_manager::{ServiceLabel, ServiceLevel, ServiceStatus};

#[test]
#[cfg_attr(
    windows,
    ignore = "installs and starts a native OS service; provide CEFARI_SERVICE_SMOKE_PROGRAM on Windows"
)]
#[cfg_attr(
    not(windows),
    ignore = "installs and starts a native OS service; run only on a disposable verification host"
)]
fn native_service_lifecycle_smoke() {
    if cfg!(windows) && env::var_os("CEFARI_SERVICE_SMOKE_PROGRAM").is_none() {
        eprintln!(
            "skipping Windows lifecycle smoke; set CEFARI_SERVICE_SMOKE_PROGRAM to a Windows-service-aware fixture binary or run with WinSW available"
        );
        return;
    }

    let spec = smoke_service_spec();
    let _winsw_fixture = prepare_windows_winsw_fixture(&spec);

    let mut manager = service_manager(None).expect("native service manager should be selectable");
    if let Some(level) = service_level() {
        manager
            .set_level(level)
            .expect("service level should be supported by native service manager");
    }

    let _cleanup = ServiceCleanup { spec: spec.clone() };
    let _ = uninstall_service(manager.as_ref(), &spec);

    install_service(manager.as_ref(), &spec).expect("service should install");
    start_service(manager.as_ref(), &spec).expect("service should start");
    wait_for_status(manager.as_ref(), &spec, |status| {
        matches!(status, ServiceStatus::Running)
    });

    stop_service(manager.as_ref(), &spec).expect("service should stop");
    wait_for_status(manager.as_ref(), &spec, |status| {
        matches!(
            status,
            ServiceStatus::Stopped(_) | ServiceStatus::NotInstalled
        )
    });

    uninstall_service(manager.as_ref(), &spec).expect("service should uninstall");
    wait_for_status(manager.as_ref(), &spec, |status| {
        matches!(status, ServiceStatus::NotInstalled)
    });
}

fn smoke_service_spec() -> CefariServiceSpec {
    let mut spec = CefariServiceSpec::new(smoke_service_label(), service_program());
    spec.args = service_args();
    spec.autostart = false;
    spec
}

fn smoke_service_label() -> ServiceLabel {
    ServiceLabel {
        qualifier: Some("dev".to_owned()),
        organization: Some("cefari".to_owned()),
        application: format!("service-smoke-{}", std::process::id()),
    }
}

fn service_program() -> OsString {
    env::var_os("CEFARI_SERVICE_SMOKE_PROGRAM").unwrap_or_else(|| OsString::from("/bin/sh"))
}

fn service_args() -> Vec<OsString> {
    if let Some(args) = env::var_os("CEFARI_SERVICE_SMOKE_ARGS") {
        let args = args
            .into_string()
            .expect("CEFARI_SERVICE_SMOKE_ARGS should be UTF-8 JSON");
        if args.trim().is_empty() {
            return default_service_args();
        }
        let parsed: Vec<String> =
            serde_json::from_str(&args).expect("CEFARI_SERVICE_SMOKE_ARGS should be a JSON array");
        return parsed.into_iter().map(OsString::from).collect();
    }

    default_service_args()
}

fn service_level() -> Option<ServiceLevel> {
    let level = env::var("CEFARI_SERVICE_SMOKE_LEVEL").ok()?;
    match level.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "system" => Some(ServiceLevel::System),
        "user" => Some(ServiceLevel::User),
        other => {
            panic!("unsupported CEFARI_SERVICE_SMOKE_LEVEL {other:?}; expected system or user")
        }
    }
}

fn default_service_args() -> Vec<OsString> {
    vec!["-c".into(), "while true; do sleep 60; done".into()]
}

#[cfg(windows)]
fn prepare_windows_winsw_fixture(spec: &CefariServiceSpec) -> Option<WindowsWinswFixture> {
    let source = env::var_os("WINSW_PATH")?;
    let source = PathBuf::from(source);
    if !source.is_file() {
        return None;
    }

    let service_name = spec.label.to_qualified_name();
    let service_dir = PathBuf::from(r"C:\ProgramData\service-manager").join(&service_name);
    fs::create_dir_all(&service_dir).expect("WinSW service directory should be creatable");

    let service_exe = service_dir.join(format!("{service_name}.exe"));
    fs::copy(&source, &service_exe).expect("WinSW fixture should copy into service directory");

    let previous_winsw_path = env::var_os("WINSW_PATH");
    unsafe {
        env::set_var("WINSW_PATH", &service_exe);
    }

    Some(WindowsWinswFixture {
        service_dir,
        previous_winsw_path,
    })
}

#[cfg(not(windows))]
fn prepare_windows_winsw_fixture(_spec: &CefariServiceSpec) -> Option<()> {
    None
}

#[cfg(windows)]
struct WindowsWinswFixture {
    service_dir: PathBuf,
    previous_winsw_path: Option<OsString>,
}

#[cfg(windows)]
impl Drop for WindowsWinswFixture {
    fn drop(&mut self) {
        if let Some(path) = &self.previous_winsw_path {
            unsafe {
                env::set_var("WINSW_PATH", path);
            }
        } else {
            unsafe {
                env::remove_var("WINSW_PATH");
            }
        }

        let _ = fs::remove_dir_all(&self.service_dir);
    }
}

fn wait_for_status(
    manager: &dyn service_manager::ServiceManager,
    spec: &CefariServiceSpec,
    expected: impl Fn(ServiceStatus) -> bool,
) {
    let timeout = Duration::from_secs(10);
    let start = Instant::now();
    let mut last_status = None;

    while start.elapsed() < timeout {
        let status = service_status(manager, spec).expect("service status should be readable");
        if expected(status.clone()) {
            return;
        }
        last_status = Some(status);
        thread::sleep(Duration::from_millis(250));
    }

    panic!("timed out waiting for service status; last status: {last_status:?}");
}

struct ServiceCleanup {
    spec: CefariServiceSpec,
}

impl Drop for ServiceCleanup {
    fn drop(&mut self) {
        if env::var_os("CEFARI_KEEP_SERVICE_SMOKE").is_some() {
            return;
        }

        if let Ok(manager) = service_manager(None) {
            let _ = stop_service(manager.as_ref(), &self.spec);
            let _ = uninstall_service(manager.as_ref(), &self.spec);
        }
    }
}
