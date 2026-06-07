use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use service_manager::{
    RestartPolicy, ServiceInstallCtx, ServiceLabel, ServiceLevel, ServiceManager,
    ServiceManagerKind, ServiceStartCtx, ServiceStatus, ServiceStatusCtx, ServiceStopCtx,
    ServiceUninstallCtx,
};

use crate::{Error, Result};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CefariServiceSpec {
    pub label: ServiceLabel,
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub working_directory: Option<PathBuf>,
    pub environment: Vec<(String, String)>,
    pub autostart: bool,
    pub level: ServiceLevel,
    pub restart_policy: RestartPolicy,
}

impl CefariServiceSpec {
    #[must_use]
    pub fn new(label: ServiceLabel, program: impl Into<PathBuf>) -> Self {
        Self {
            label,
            program: program.into(),
            args: Vec::new(),
            working_directory: None,
            environment: Vec::new(),
            autostart: true,
            level: default_service_level(),
            restart_policy: RestartPolicy::default(),
        }
    }

    #[must_use]
    pub fn daemon(program: impl Into<PathBuf>) -> Self {
        Self::new(
            ServiceLabel {
                qualifier: Some("dev".to_owned()),
                organization: Some("cefari".to_owned()),
                application: "daemon".to_owned(),
            },
            program,
        )
    }

    #[must_use]
    pub fn with_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn with_working_directory(mut self, working_directory: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(working_directory.into());
        self
    }

    #[must_use]
    pub fn with_environment(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    #[must_use]
    pub fn install_context(&self) -> ServiceInstallCtx {
        ServiceInstallCtx {
            label: self.label.clone(),
            program: self.program.clone(),
            args: self.args.clone(),
            contents: None,
            username: None,
            working_directory: self.working_directory.clone(),
            environment: (!self.environment.is_empty()).then(|| self.environment.clone()),
            autostart: self.autostart,
            restart_policy: self.restart_policy,
        }
    }

    #[must_use]
    pub fn uninstall_context(&self) -> ServiceUninstallCtx {
        ServiceUninstallCtx {
            label: self.label.clone(),
        }
    }

    #[must_use]
    pub fn start_context(&self) -> ServiceStartCtx {
        ServiceStartCtx {
            label: self.label.clone(),
        }
    }

    #[must_use]
    pub fn stop_context(&self) -> ServiceStopCtx {
        ServiceStopCtx {
            label: self.label.clone(),
        }
    }

    #[must_use]
    pub fn status_context(&self) -> ServiceStatusCtx {
        ServiceStatusCtx {
            label: self.label.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ServiceOperation {
    Install,
    Uninstall,
    Start,
    Stop,
    Restart,
    Status,
}

impl ServiceOperation {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Status => "status",
        }
    }
}

pub fn service_manager(
    kind: impl Into<Option<ServiceManagerKind>>,
) -> Result<Box<dyn ServiceManager>> {
    let mut manager =
        <dyn ServiceManager>::target_or_native(kind).map_err(|source| Error::ServiceManager {
            operation: "select",
            source,
        })?;
    manager
        .set_level(default_service_level())
        .map_err(|source| Error::ServiceManager {
            operation: "set-level",
            source,
        })?;
    Ok(manager)
}

#[must_use]
pub fn default_service_level() -> ServiceLevel {
    if cfg!(target_os = "windows") {
        ServiceLevel::System
    } else {
        ServiceLevel::User
    }
}

pub fn install_service(manager: &dyn ServiceManager, spec: &CefariServiceSpec) -> Result<()> {
    manager
        .install(spec.install_context())
        .map_err(|source| Error::ServiceManager {
            operation: ServiceOperation::Install.as_str(),
            source,
        })
}

pub fn uninstall_service(manager: &dyn ServiceManager, spec: &CefariServiceSpec) -> Result<()> {
    manager
        .uninstall(spec.uninstall_context())
        .map_err(|source| Error::ServiceManager {
            operation: ServiceOperation::Uninstall.as_str(),
            source,
        })
}

pub fn start_service(manager: &dyn ServiceManager, spec: &CefariServiceSpec) -> Result<()> {
    manager
        .start(spec.start_context())
        .map_err(|source| Error::ServiceManager {
            operation: ServiceOperation::Start.as_str(),
            source,
        })
}

pub fn stop_service(manager: &dyn ServiceManager, spec: &CefariServiceSpec) -> Result<()> {
    manager
        .stop(spec.stop_context())
        .map_err(|source| Error::ServiceManager {
            operation: ServiceOperation::Stop.as_str(),
            source,
        })
}

pub fn restart_service(manager: &dyn ServiceManager, spec: &CefariServiceSpec) -> Result<()> {
    stop_service(manager, spec)?;
    start_service(manager, spec)
}

pub fn service_status(
    manager: &dyn ServiceManager,
    spec: &CefariServiceSpec,
) -> Result<ServiceStatus> {
    manager
        .status(spec.status_context())
        .map_err(|source| Error::ServiceManager {
            operation: ServiceOperation::Status.as_str(),
            source,
        })
}

#[must_use]
pub fn program_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, ffi::OsString, io, path::PathBuf};

    use service_manager::{
        ServiceInstallCtx, ServiceLevel, ServiceManager, ServiceStartCtx, ServiceStatus,
        ServiceStatusCtx, ServiceStopCtx, ServiceUninstallCtx,
    };

    use super::{
        CefariServiceSpec, ServiceOperation, default_service_level, install_service,
        program_exists, restart_service, service_manager, service_status, start_service,
        stop_service, uninstall_service,
    };

    #[test]
    fn builds_daemon_install_context() {
        let spec = CefariServiceSpec::daemon("/usr/local/bin/cefari-daemon")
            .with_arg("--foreground")
            .with_working_directory("/tmp")
            .with_environment("CEFARI_ENV", "test");

        let context = spec.install_context();

        assert_eq!(context.label.to_qualified_name(), "dev.cefari.daemon");
        assert_eq!(
            context.program,
            PathBuf::from("/usr/local/bin/cefari-daemon")
        );
        assert_eq!(context.args, vec![OsString::from("--foreground")]);
        assert_eq!(context.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(
            context.environment,
            Some(vec![("CEFARI_ENV".to_owned(), "test".to_owned())])
        );
        assert!(context.autostart);
    }

    #[test]
    fn defaults_to_platform_service_level() {
        let spec = CefariServiceSpec::daemon("/usr/local/bin/cefari-daemon");

        assert_eq!(spec.level, default_service_level());
    }

    #[test]
    fn default_manager_level_matches_platform_support() {
        if cfg!(target_os = "windows") {
            assert_eq!(default_service_level(), ServiceLevel::System);
        } else {
            assert_eq!(default_service_level(), ServiceLevel::User);
        }
    }

    #[test]
    fn native_service_manager_uses_supported_default_level() {
        let manager = service_manager(None).expect("native service manager should be selectable");

        assert_eq!(manager.level(), default_service_level());
    }

    #[test]
    fn exposes_service_operation_names() {
        assert_eq!(ServiceOperation::Restart.as_str(), "restart");
        assert_eq!(ServiceOperation::Status.as_str(), "status");
    }

    #[test]
    fn checks_program_paths_without_side_effects() {
        assert!(program_exists("/bin/sh"));
        assert!(!program_exists("/definitely/not/a/cefari-daemon"));
    }

    #[test]
    fn service_helpers_dispatch_expected_manager_operations() {
        let manager = RecordingServiceManager::default();
        let spec = CefariServiceSpec::daemon("/usr/local/bin/cefari-daemon");

        install_service(&manager, &spec).expect("install should dispatch");
        start_service(&manager, &spec).expect("start should dispatch");
        assert_eq!(
            service_status(&manager, &spec).expect("status should dispatch"),
            ServiceStatus::Running
        );
        stop_service(&manager, &spec).expect("stop should dispatch");
        restart_service(&manager, &spec).expect("restart should dispatch");
        uninstall_service(&manager, &spec).expect("uninstall should dispatch");

        assert_eq!(
            manager.operations.borrow().as_slice(),
            [
                "install:dev.cefari.daemon",
                "start:dev.cefari.daemon",
                "status:dev.cefari.daemon",
                "stop:dev.cefari.daemon",
                "stop:dev.cefari.daemon",
                "start:dev.cefari.daemon",
                "uninstall:dev.cefari.daemon",
            ]
        );
    }

    #[derive(Default)]
    struct RecordingServiceManager {
        operations: RefCell<Vec<String>>,
    }

    impl RecordingServiceManager {
        fn record(&self, operation: &str, label: &service_manager::ServiceLabel) {
            self.operations
                .borrow_mut()
                .push(format!("{operation}:{}", label.to_qualified_name()));
        }
    }

    impl ServiceManager for RecordingServiceManager {
        fn available(&self) -> io::Result<bool> {
            Ok(true)
        }

        fn install(&self, ctx: ServiceInstallCtx) -> io::Result<()> {
            self.record("install", &ctx.label);
            Ok(())
        }

        fn uninstall(&self, ctx: ServiceUninstallCtx) -> io::Result<()> {
            self.record("uninstall", &ctx.label);
            Ok(())
        }

        fn start(&self, ctx: ServiceStartCtx) -> io::Result<()> {
            self.record("start", &ctx.label);
            Ok(())
        }

        fn stop(&self, ctx: ServiceStopCtx) -> io::Result<()> {
            self.record("stop", &ctx.label);
            Ok(())
        }

        fn level(&self) -> ServiceLevel {
            ServiceLevel::User
        }

        fn set_level(&mut self, _level: ServiceLevel) -> io::Result<()> {
            Ok(())
        }

        fn status(&self, ctx: ServiceStatusCtx) -> io::Result<ServiceStatus> {
            self.record("status", &ctx.label);
            Ok(ServiceStatus::Running)
        }
    }
}
