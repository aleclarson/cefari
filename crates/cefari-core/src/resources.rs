use std::path::{Component, Path, PathBuf};

pub use cargo_packager_resource_resolver::PackageFormat;

use crate::{Error, Result};

pub fn packaged_resources_dir(format: PackageFormat) -> Result<PathBuf> {
    cargo_packager_resource_resolver::resources_dir(format)
        .map_err(|source| Error::ResolveResources { source })
}

pub fn resolve_resource(
    resource_dir: impl AsRef<Path>,
    resource_path: impl AsRef<Path>,
) -> Result<PathBuf> {
    let resource_dir = resource_dir.as_ref();
    let resource_path = resource_path.as_ref();

    validate_relative_resource_path(resource_path)?;

    let resolved = resource_dir.join(resource_path);
    if resolved.exists() {
        Ok(resolved)
    } else {
        Err(Error::MissingResource { path: resolved })
    }
}

fn validate_relative_resource_path(path: &Path) -> Result<()> {
    let is_valid = path
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));

    if path.is_relative() && is_valid {
        Ok(())
    } else {
        Err(Error::InvalidResourcePath {
            path: path.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{Error, resolve_resource};

    #[test]
    fn resolves_existing_relative_resource() {
        let root = temp_dir();
        fs::create_dir_all(root.join("ui")).expect("resource dir should be created");
        fs::write(root.join("ui/index.html"), "<!doctype html>")
            .expect("resource should be written");

        let resource = resolve_resource(&root, "ui/index.html").expect("resource should resolve");
        assert_eq!(resource, root.join("ui/index.html"));

        fs::remove_dir_all(root).expect("temp dir should be removed");
    }

    #[test]
    fn rejects_parent_resource_path() {
        let root = temp_dir();
        let error = resolve_resource(&root, "../secret").expect_err("parent path should fail");

        assert!(matches!(error, Error::InvalidResourcePath { .. }));
    }

    #[test]
    fn reports_missing_resource() {
        let root = temp_dir();
        let error =
            resolve_resource(&root, "missing.txt").expect_err("missing resource should fail");

        assert!(matches!(error, Error::MissingResource { .. }));
    }

    fn temp_dir() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-core-resource-test-{suffix}"))
    }
}
