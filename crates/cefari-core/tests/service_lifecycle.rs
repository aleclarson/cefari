use std::{
    env, thread,
    time::{Duration, Instant},
};

use cefari_core::{
    CefariServiceSpec, install_service, service_manager, service_status, start_service,
    stop_service, uninstall_service,
};
use service_manager::{ServiceLabel, ServiceStatus};

#[test]
#[cfg_attr(
    windows,
    ignore = "requires a Windows-service-aware fixture binary or WinSW"
)]
#[cfg_attr(
    not(windows),
    ignore = "installs and starts a native OS service; run only on a disposable verification host"
)]
fn native_service_lifecycle_smoke() {
    if cfg!(windows) {
        eprintln!("skipping Windows lifecycle smoke until a service-aware fixture is available");
        return;
    }

    let manager = service_manager(None).expect("native service manager should be selectable");
    let spec = smoke_service_spec();

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

fn service_program() -> &'static str {
    "/bin/sh"
}

fn service_args() -> Vec<std::ffi::OsString> {
    vec!["-c".into(), "while true; do sleep 60; done".into()]
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
