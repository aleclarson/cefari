use std::path::Path;

use thiserror::Error;

use super::{ProjectCapability, ProjectConfig};

pub(super) fn validate_project_config(
    project: &ProjectConfig,
) -> Result<(), ProjectConfigValidationError> {
    validate_project_name(&project.app.project_name)?;
    validate_required_string("app.name", &project.app.name)?;
    validate_required_string("app.identifier", &project.app.identifier)?;
    validate_optional_relative_path("app.icon", project.app.icon.as_deref())?;
    let mut tray_count = 0;
    for capability in &project.capabilities {
        match capability {
            ProjectCapability::Tray { icon } => {
                tray_count += 1;
                let Some(icon) = icon.as_deref() else {
                    return Err(ProjectConfigValidationError::new(
                        "capabilities[].icon",
                        "is required for tray capabilities",
                    ));
                };
                validate_relative_path("capabilities[].icon", icon)?;
            }
        }
    }
    if tray_count > 1 {
        return Err(ProjectConfigValidationError::new(
            "capabilities",
            "must not include more than one tray capability",
        ));
    }
    validate_relative_path("frontend.dist", &project.frontend.dist)?;
    validate_command(
        "frontend.buildCommand",
        project.frontend.build_command.as_deref(),
    )?;
    validate_command(
        "frontend.devCommand",
        project.frontend.dev_command.as_deref(),
    )?;
    if project.frontend.dev_port == 0 && project.frontend.dev_command.is_some() {
        return Err(ProjectConfigValidationError::new(
            "frontend.devPort",
            "must be greater than 0 when frontend.devCommand is configured",
        ));
    }
    validate_relative_path("daemon.entry", &project.daemon.entry)?;
    validate_required_string("package.productName", &project.package.product_name)?;
    validate_version("package.version", &project.package.version)?;
    Ok(())
}

fn validate_project_name(value: &str) -> Result<(), ProjectConfigValidationError> {
    if is_valid_project_name(value) {
        Ok(())
    } else {
        Err(ProjectConfigValidationError::new(
            "app.projectName",
            "must match ^[a-z0-9-]+$ and cannot be empty",
        ))
    }
}

fn is_valid_project_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_required_string(
    field: &'static str,
    value: &str,
) -> Result<(), ProjectConfigValidationError> {
    if value.trim().is_empty() {
        Err(ProjectConfigValidationError::new(
            field,
            "must not be blank",
        ))
    } else {
        Ok(())
    }
}

fn validate_optional_relative_path(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ProjectConfigValidationError> {
    value.map_or(Ok(()), |value| validate_relative_path(field, value))
}

fn validate_relative_path(
    field: &'static str,
    value: &str,
) -> Result<(), ProjectConfigValidationError> {
    validate_required_string(field, value)?;
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ProjectConfigValidationError::new(
            field,
            "must be a relative path inside the project",
        ));
    }
    Ok(())
}

fn validate_command(
    field: &'static str,
    value: Option<&[String]>,
) -> Result<(), ProjectConfigValidationError> {
    let Some(command) = value else {
        return Ok(());
    };
    if command.is_empty() {
        return Err(ProjectConfigValidationError::new(
            field,
            "must contain at least one argument",
        ));
    }
    if command.iter().any(|argument| argument.trim().is_empty()) {
        return Err(ProjectConfigValidationError::new(
            field,
            "must contain only non-blank arguments",
        ));
    }
    Ok(())
}

fn validate_version(field: &'static str, value: &str) -> Result<(), ProjectConfigValidationError> {
    validate_required_string(field, value)?;
    if semver::Version::parse(value).is_err() {
        return Err(ProjectConfigValidationError::new(
            field,
            "must be a valid semantic version",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
#[error("{field} {message}")]
pub struct ProjectConfigValidationError {
    field: &'static str,
    message: &'static str,
}

impl ProjectConfigValidationError {
    pub(crate) fn new(field: &'static str, message: &'static str) -> Self {
        Self { field, message }
    }
}
