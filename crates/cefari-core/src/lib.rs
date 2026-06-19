//! Reusable runtime support for Cefari desktop applications.
//!
//! `cefari-core` owns runtime-safe helpers for paths, configuration, resource
//! lookup, logging inputs, updates, and service management. Windowing, CEF
//! initialization, and developer orchestration stay outside this crate.

pub mod config;
pub mod ipc;
pub mod logging;
pub mod paths;
pub mod resources;
pub mod services;
pub mod updates;

mod error;

pub use config::{
    AppConfig, BrowserConfig, CefariConfig, DaemonConfig, DeepLinkConfig, ServiceConfig,
    UpdateConfig, load_config, save_config,
};
pub use error::{Error, Result};
pub use ipc::*;
pub use logging::{
    CEFARI_DAEMON_LOG_ENV, LogFileConfig, LogFormat, LogRotation, LogStream, RuntimeLogConfig,
    prune_rotated_logs,
};
pub use paths::{AppIdentity, RuntimePaths};
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
