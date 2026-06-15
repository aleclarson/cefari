use std::{
    path::{Path, PathBuf},
    process::Command,
};

use super::{CONFIG_FILE_NAME, loader::LoadProjectError};

const DENO_ENV: &str = "CEFARI_DENO";
const EXPECTED_DENO_MAJOR: u64 = 2;
const EXPECTED_DENO_MINOR: u64 = 8;

pub(super) fn deno_command() -> PathBuf {
    std::env::var_os(DENO_ENV).map_or_else(|| PathBuf::from("deno"), PathBuf::from)
}

pub(super) fn warn_if_deno_is_older_than_expected(deno: &Path) -> Result<(), LoadProjectError> {
    let output = Command::new(deno)
        .arg("--version")
        .output()
        .map_err(|source| LoadProjectError::DenoMissing { source })?;

    if !output.status.success() {
        return Err(LoadProjectError::DenoFailed {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(version) = parse_deno_version(&stdout)
        && is_older_than_expected_deno(version)
    {
        eprintln!(
            "warning: Cefari expects Deno {EXPECTED_DENO_MAJOR}.{EXPECTED_DENO_MINOR}+ to load {CONFIG_FILE_NAME}; found Deno {}.{}.{}",
            version.major, version.minor, version.patch
        );
    }

    Ok(())
}

#[must_use]
pub fn deno_status() -> DenoStatus {
    match Command::new(deno_command()).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_deno_version(&stdout).map_or_else(
                || DenoStatus::Unknown {
                    output: stdout.lines().next().unwrap_or_default().to_owned(),
                },
                |version| {
                    if is_older_than_expected_deno(version) {
                        DenoStatus::Older { version }
                    } else {
                        DenoStatus::Expected { version }
                    }
                },
            )
        }
        Ok(output) => DenoStatus::Failed {
            status: output.status.to_string(),
        },
        Err(_) => DenoStatus::Missing,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DenoStatus {
    Expected { version: DenoVersion },
    Older { version: DenoVersion },
    Unknown { output: String },
    Failed { status: String },
    Missing,
}

impl DenoStatus {
    #[must_use]
    pub fn doctor_message(&self) -> String {
        match self {
            Self::Expected { version } => format!("{version} OK"),
            Self::Older { version } => {
                format!("{version} older than expected; Cefari expects Deno 2.8+")
            }
            Self::Unknown { output } => format!("found, version unrecognized ({output})"),
            Self::Failed { status } => format!("failed ({status})"),
            Self::Missing => "missing".to_owned(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DenoVersion {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

impl std::fmt::Display for DenoVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub(crate) fn parse_deno_version(output: &str) -> Option<DenoVersion> {
    let line = output.lines().find(|line| line.starts_with("deno "))?;
    let version = line.strip_prefix("deno ")?;
    let mut parts = version.split_whitespace().next()?.split('.');
    Some(DenoVersion {
        major: parts.next()?.parse().ok()?,
        minor: parts.next()?.parse().ok()?,
        patch: parts.next()?.parse().ok()?,
    })
}

pub(crate) fn is_older_than_expected_deno(version: DenoVersion) -> bool {
    (version.major, version.minor) < (EXPECTED_DENO_MAJOR, EXPECTED_DENO_MINOR)
}
