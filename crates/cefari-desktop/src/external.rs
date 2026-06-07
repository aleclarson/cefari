use std::path::Path;

use anyhow::{Context, Result};

pub fn open_external_url(url: &str) -> Result<()> {
    if !is_supported_external_url(url) {
        anyhow::bail!("unsupported external URL scheme: {url}");
    }

    open::that_detached(url).with_context(|| format!("failed to open external URL: {url}"))
}

pub fn open_external_file(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("external file does not exist: {}", path.display());
    }

    open::that_detached(path).with_context(|| {
        format!(
            "failed to open external file or directory: {}",
            path.display()
        )
    })
}

fn is_supported_external_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")
}

#[cfg(test)]
mod tests {
    use super::{is_supported_external_url, open_external_file, open_external_url};

    #[test]
    fn accepts_expected_external_url_schemes() {
        assert!(is_supported_external_url("https://example.test"));
        assert!(is_supported_external_url("http://example.test"));
        assert!(is_supported_external_url("mailto:hello@example.test"));
    }

    #[test]
    fn rejects_unsupported_external_url_schemes_before_opening() {
        let error = open_external_url("file:///etc/passwd").expect_err("file URL should fail");

        assert!(
            error
                .to_string()
                .contains("unsupported external URL scheme")
        );
    }

    #[test]
    fn rejects_missing_external_files_before_opening() {
        let missing = std::env::temp_dir().join("cefari-missing-external-file");
        let error = open_external_file(&missing).expect_err("missing file should fail");

        assert!(error.to_string().contains("external file does not exist"));
    }
}
