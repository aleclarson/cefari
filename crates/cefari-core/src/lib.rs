//! Reusable runtime support for Cefari desktop applications.
//!
//! `cefari-core` owns runtime-safe helpers for paths, configuration, resource
//! lookup, logging inputs, updates, and service management. Windowing, CEF
//! initialization, and developer orchestration stay outside this crate.

pub mod config;
pub mod ipc;
pub mod logging;
pub mod paths;
pub mod platform;
pub mod resources;
pub mod services;
pub mod updates;

mod error;

pub use config::{
    AppConfig, BrowserConfig, CefariConfig, DaemonConfig, DeepLinkConfig, ServiceConfig,
    NativeResourceConfig, UpdateConfig, WorkerConfig, WorkerDenoSourceConfig, WorkerEntryConfig,
    WorkerExecutableConfig, WorkerPermissionConfig,
    WorkerPermissionsConfig, WorkerTargetConfig, load_config, save_config,
};
pub use error::{Error, Result};
pub use ipc::*;
pub use logging::{
    APP_LOG_SCOPE, CEFARI_DAEMON_LOG_ENV, CEFARI_LOG_DATABASE_ENV, CEFARI_LOG_DATABASE_FILE_NAME,
    CEFARI_LOG_SCHEMA_SQL, CEFARI_LOG_SCHEMA_VERSION, CEFARI_LOG_SCOPE, DAEMON_LOG_SCOPE,
    LOG_PROPERTY_COMPONENT, LOG_PROPERTY_CONNECTION_ID, LOG_PROPERTY_WINDOW_ID,
    LOG_PROPERTY_WORKER, LOG_PROPERTY_WORKER_ID, LogDatabaseConfig, LogFileConfig, LogFormat,
    LogRotation, LogScope, LogStream, RuntimeLogConfig, WORKER_LOG_SCOPE_PREFIX,
    prune_rotated_logs, should_redact_log_property_key, worker_log_scope,
};
pub use paths::{AppIdentity, RuntimePaths};
pub use platform::{CefariTarget, PlatformSupport};
pub use resources::{PackageFormat, packaged_resources_dir, resolve_resource};
pub use services::{
    CefariServiceSpec, ServiceOperation, default_service_level, install_service, program_exists,
    restart_service, service_manager, service_status, start_service, stop_service,
    uninstall_service,
};
pub use updates::{
    AvailableUpdate, PendingUpdate, PreparedUpdateCheck, UpdateCheckConfig, UpdateCheckState,
    check_for_update, install_update, update_id,
};
