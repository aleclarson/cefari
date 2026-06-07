use std::{io, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "could not resolve platform project directories for qualifier={qualifier}, organization={organization}, application={application}"
    )]
    ProjectDirectoriesUnavailable {
        qualifier: String,
        organization: String,
        application: String,
    },

    #[error("failed to read config at {path}")]
    ReadConfig { path: PathBuf, source: io::Error },

    #[error("failed to parse config at {path}")]
    ParseConfig {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to create config directory at {path}")]
    CreateConfigDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write config at {path}")]
    WriteConfig { path: PathBuf, source: io::Error },

    #[error("failed to resolve packaged resource directory")]
    ResolveResources {
        source: cargo_packager_resource_resolver::Error,
    },

    #[error(
        "resource path must be relative and cannot contain parent directory components: {path}"
    )]
    InvalidResourcePath { path: PathBuf },

    #[error("resource does not exist at {path}")]
    MissingResource { path: PathBuf },

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
