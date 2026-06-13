use std::{fs, path::Path, path::PathBuf, process::Command};

use anyhow::{Context, Result};

use crate::{cef, project::ProjectConfig, runtime::resolve_desktop_runtime};

pub(crate) fn daemon_executable_name(project: &ProjectConfig) -> String {
    platform_executable_name(&format!("{}-daemon", project.app.project_name))
}

pub(crate) fn desktop_executable_name(project: &ProjectConfig) -> String {
    platform_executable_name(&project.app.project_name)
}

fn platform_executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

pub fn build_project(project_dir: &Path, release: bool) -> Result<()> {
    let project = ProjectConfig::load_from_dir(project_dir)?;
    let build_dir = ProjectConfig::build_dir(project_dir);
    let frontend_out = build_dir.join("frontend");
    let daemon_out = build_dir.join("daemon");
    let desktop_out = build_dir.join("desktop");

    fs::create_dir_all(&frontend_out).with_context(|| {
        format!(
            "failed to create frontend build directory at {}",
            frontend_out.display()
        )
    })?;
    fs::create_dir_all(&daemon_out).with_context(|| {
        format!(
            "failed to create daemon build directory at {}",
            daemon_out.display()
        )
    })?;
    fs::create_dir_all(&desktop_out).with_context(|| {
        format!(
            "failed to create desktop build directory at {}",
            desktop_out.display()
        )
    })?;

    build_frontend(project_dir, &project, &frontend_out)?;
    build_daemon(project_dir, &project, &daemon_out)?;
    let cef = cef::prepare_cef(project_dir)?;
    println!("prepared CEF resources at {}", cef.resources_dir.display());
    println!("cached CEF downloads at {}", cef.cache_dir.display());
    println!(
        "verified CEF archive metadata at {}",
        cef.archive_json.display()
    );
    build_desktop(&project, &desktop_out, release)?;

    println!("built Cefari project at {}", project_dir.display());
    Ok(())
}

fn build_frontend(project_dir: &Path, project: &ProjectConfig, output_dir: &Path) -> Result<()> {
    if let Some(command) = &project.frontend.build_command {
        run_frontend_build_command(project_dir, command)?;
        copy_frontend_dist(project_dir, project, output_dir)?;
        return Ok(());
    }

    let source = project_dir.join("frontend/index.html");
    let output = output_dir.join("index.html");
    fs::copy(&source, &output).with_context(|| {
        format!(
            "failed to copy frontend entry from {} to {}",
            source.display(),
            output.display()
        )
    })?;

    let configured_dist = project_dir.join(&project.frontend.dist);
    fs::create_dir_all(&configured_dist).with_context(|| {
        format!(
            "failed to create configured frontend dist at {}",
            configured_dist.display()
        )
    })?;
    fs::copy(&source, configured_dist.join("index.html")).with_context(|| {
        format!(
            "failed to copy frontend entry into configured dist at {}",
            configured_dist.display()
        )
    })?;

    Ok(())
}

fn run_frontend_build_command(project_dir: &Path, command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("frontend build_command must contain at least one argument");
    }

    let status = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(project_dir)
        .status()
        .with_context(|| format!("failed to run frontend build command {}", command[0]))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("frontend build command failed with status {status}");
    }
}

fn copy_frontend_dist(
    project_dir: &Path,
    project: &ProjectConfig,
    output_dir: &Path,
) -> Result<()> {
    let configured_dist = project_dir.join(&project.frontend.dist);
    if !configured_dist.is_dir() {
        anyhow::bail!(
            "frontend dist directory {} does not exist after build command",
            configured_dist.display()
        );
    }

    if output_dir.exists() {
        fs::remove_dir_all(output_dir).with_context(|| {
            format!(
                "failed to remove previous frontend build directory at {}",
                output_dir.display()
            )
        })?;
    }
    copy_dir_recursive(&configured_dist, output_dir).with_context(|| {
        format!(
            "failed to copy frontend dist from {} to {}",
            configured_dist.display(),
            output_dir.display()
        )
    })
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).with_context(|| {
        format!(
            "failed to create destination directory at {}",
            destination.display()
        )
    })?;

    for entry in fs::read_dir(source)
        .with_context(|| format!("failed to read source directory at {}", source.display()))?
    {
        let entry = entry
            .with_context(|| format!("failed to read source entry at {}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for {}", source_path.display()))?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "failed to copy frontend file from {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn build_daemon(project_dir: &Path, project: &ProjectConfig, output_dir: &Path) -> Result<()> {
    let source = project_dir.join(&project.daemon.entry);
    let source_copy = output_dir.join("main.ts");
    fs::copy(&source, &source_copy).with_context(|| {
        format!(
            "failed to copy daemon entry from {} to {}",
            source.display(),
            source_copy.display()
        )
    })?;

    let executable = output_dir.join(daemon_executable_name(project));
    let status = Command::new("deno")
        .arg("compile")
        .arg("--allow-read")
        .arg("--allow-net")
        .arg("--output")
        .arg(&executable)
        .arg(&source)
        .status()
        .context("failed to run deno compile for Cefari daemon")?;

    if !status.success() {
        anyhow::bail!("deno compile failed with status {status}");
    }

    Ok(())
}

fn build_desktop(project: &ProjectConfig, output_dir: &Path, release: bool) -> Result<()> {
    let source = resolve_desktop_runtime(release)?;
    let destination = output_dir.join(desktop_executable_name(project));
    fs::copy(&source, &destination).with_context(|| {
        format!(
            "failed to copy desktop executable from {} to {}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}

pub(crate) fn workspace_manifest() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("cefari-cli should live under crates/cefari-cli")
        .join("Cargo.toml")
}

pub(crate) fn workspace_target_dir(release: bool) -> PathBuf {
    workspace_manifest()
        .parent()
        .expect("workspace manifest should have a parent")
        .join(if release {
            "target/release"
        } else {
            "target/debug"
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::project::ProjectConfig;

    use super::{
        copy_frontend_dist, daemon_executable_name, desktop_executable_name, workspace_manifest,
    };
    use crate::runtime::configure_desktop_build_command;

    #[test]
    fn daemon_executable_name_matches_host_platform() {
        let project = project_config();

        if cfg!(windows) {
            assert_eq!(daemon_executable_name(&project), "example-app-daemon.exe");
        } else {
            assert_eq!(daemon_executable_name(&project), "example-app-daemon");
        }
    }

    #[test]
    fn desktop_executable_name_matches_host_platform() {
        let project = project_config();

        if cfg!(windows) {
            assert_eq!(desktop_executable_name(&project), "example-app.exe");
        } else {
            assert_eq!(desktop_executable_name(&project), "example-app");
        }
    }

    #[test]
    fn resolves_workspace_manifest() {
        assert!(workspace_manifest().ends_with("Cargo.toml"));
        assert!(workspace_manifest().exists());
    }

    #[test]
    fn desktop_build_command_targets_desktop_crate() {
        let mut command = std::process::Command::new("cargo");

        configure_desktop_build_command(&mut command);

        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.windows(2).any(|args| args == ["-p", "cefari-desktop"]));
        assert!(!args.iter().any(|arg| arg == "--features"));
    }

    #[test]
    fn copies_configured_frontend_dist_recursively() {
        let root =
            std::env::temp_dir().join(format!("cefari-frontend-dist-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("frontend/dist/assets")).expect("dist should be created");
        fs::write(root.join("frontend/dist/index.html"), "<!doctype html>")
            .expect("index should be written");
        fs::write(
            root.join("frontend/dist/assets/app.js"),
            "console.log('ok')",
        )
        .expect("asset should be written");

        let project = project_config();
        let output = root.join("build/frontend");

        copy_frontend_dist(&root, &project, &output).expect("dist should copy");

        assert!(output.join("index.html").exists());
        assert!(output.join("assets/app.js").exists());

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    fn project_config() -> ProjectConfig {
        toml::from_str(
            r#"[app]
project_name = "example-app"
name = "Example App"
identifier = "dev.cefari.example-app"

[frontend]
dist = "frontend/dist"

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "Example App"
"#,
        )
        .expect("project should parse")
    }
}
