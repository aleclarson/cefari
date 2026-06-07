//! Reusable runtime support for Cefari desktop applications.
//!
//! `cefari-core` owns runtime-safe helpers for paths, configuration, resource
//! lookup, logging inputs, updates, and service management. Windowing, CEF
//! initialization, and developer orchestration stay outside this crate.

pub mod config;
pub mod logging;
pub mod paths;
pub mod resources;

mod error;

pub use config::{AppConfig, CefariConfig, ServiceConfig, UpdateConfig, load_config, save_config};
pub use error::{Error, Result};
pub use logging::{LogFormat, RuntimeLogConfig};
pub use paths::{AppIdentity, RuntimePaths};
pub use resources::{PackageFormat, packaged_resources_dir, resolve_resource};
