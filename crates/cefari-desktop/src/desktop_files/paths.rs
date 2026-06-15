use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};

pub(super) fn path_to_utf8(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}

pub(super) fn checked_path(path: &str) -> Result<&str> {
    if path.is_empty() || path == "." {
        return Ok(".");
    }

    let path = Path::new(path);
    if path.is_absolute() {
        anyhow::bail!("absolute paths are not allowed");
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                anyhow::bail!("parent path traversal is not allowed")
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                anyhow::bail!("absolute paths are not allowed");
            }
        }
    }

    path_to_utf8(path)
}

pub(super) fn child_path(parent: &str, name: &str) -> String {
    if parent == "." {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

pub(super) fn temporary_sibling_path(path: &str) -> Result<String> {
    let path = Path::new(path);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("file path must end with a UTF-8 file name"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_nanos();
    let temporary_name = format!(".{file_name}.cefari-tmp-{nonce}");

    Ok(path.parent().map_or_else(
        || temporary_name.clone(),
        |parent| {
            if parent.as_os_str().is_empty() {
                temporary_name.clone()
            } else {
                parent.join(&temporary_name).to_string_lossy().into_owned()
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::checked_path;

    #[test]
    fn rejects_parent_traversal_paths() {
        let error = checked_path("../secret.txt").expect_err("traversal should fail");

        assert!(error.to_string().contains("parent path traversal"));
    }
}
