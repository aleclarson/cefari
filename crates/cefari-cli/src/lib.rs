use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

mod build;
mod cef;
mod clean;
mod dev;
mod logs;
mod package;
pub mod project;
mod release;
mod runtime;

use project::ProjectConfig;

#[derive(Debug, Parser)]
#[command(name = "cefari")]
#[command(version)]
#[command(about = "Create, develop, build, package, sign, and release Cefari apps.")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new Cefari project.
    Init {
        /// Directory to create. Defaults to ./cefari-app.
        #[arg(default_value = "cefari-app")]
        path: PathBuf,

        /// Application display name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Run the local development environment.
    Dev {
        /// Project directory to run. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Override the frontend dev server port. Use 0 with the built-in static server to request any free port.
        #[arg(long)]
        frontend_port: Option<u16>,

        /// Chrome `DevTools` Protocol port for the embedded CEF browser. Defaults to any free local port.
        #[arg(long)]
        devtools_port: Option<u16>,
    },
    /// Build frontend, daemon, and desktop artifacts.
    Build {
        /// Project directory to build. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Build the desktop runtime with Cargo's release profile.
        #[arg(long)]
        release: bool,
    },
    /// Package a built Cefari app.
    Package {
        /// Project directory to package. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Package the desktop runtime from Cargo's release profile.
        #[arg(long)]
        release: bool,

        /// Version to write into native package metadata. Defaults to the Cefari CLI version.
        #[arg(long)]
        release_version: Option<String>,
    },
    /// Code sign a packaged app.
    Codesign {
        /// Artifact to sign.
        artifact: PathBuf,

        /// Platform signing flow to use.
        #[arg(long, value_enum, default_value_t = release::SignPlatform::current())]
        platform: release::SignPlatform,

        /// Path to sign.toml.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Notarize a signed app.
    Notarize {
        /// macOS .app or .dmg artifact to notarize.
        artifact: PathBuf,

        /// Path to sign.toml.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Generate update artifacts.
    MakeUpdate {
        /// Release archive to sign for update installation.
        archive: PathBuf,

        /// Public download URL for the release archive.
        #[arg(long)]
        url: String,

        /// Version advertised to the runtime updater.
        #[arg(long)]
        version: String,

        /// Updater target key, such as darwin-aarch64 or linux-x86_64.
        #[arg(long, default_value_t = release::default_update_target())]
        target: String,

        /// Updater package format. Defaults from --target when omitted.
        #[arg(long, value_enum)]
        format: Option<release::UpdatePackageFormat>,

        /// Env var read by cargo-codesign for the update signing key.
        #[arg(long, default_value = "UPDATE_SIGNING_KEY")]
        key_env: String,

        /// Directory where update metadata and signatures are written.
        #[arg(long, default_value = "dist/update")]
        output_dir: PathBuf,
    },
    /// Remove generated build and dist artifacts.
    Clean {
        /// Project directory to clean. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Check local tool and project health.
    Doctor,
    /// Print Cefari app, daemon, and Rust runtime logs for debugging.
    Logs {
        /// Log stream to read.
        #[arg(long, value_enum, default_value_t = logs::LogKind::All)]
        kind: logs::LogKind,

        /// Number of recent lines to print per stream. Use 0 for all lines.
        #[arg(long, default_value_t = 200)]
        tail: usize,

        /// Keep printing appended log output.
        #[arg(long)]
        follow: bool,

        /// Print the Cefari log directory and exit.
        #[arg(long)]
        path: bool,
    },
    /// Print environment and project information.
    Info,
}

pub fn run() -> Result<()> {
    run_from(std::env::args_os())
}

pub fn run_from<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    run_command(cli.command)
}

pub fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Init { path, name } => init_project(&path, name.as_deref()),
        Command::Dev {
            path,
            frontend_port,
            devtools_port,
        } => dev::dev_project(&path, frontend_port, devtools_port),
        Command::Build { path, release } => build::build_project(&path, release),
        Command::Package {
            path,
            release,
            release_version,
        } => package::package_project(&path, release, release_version.as_deref()),
        Command::Codesign {
            artifact,
            platform,
            config,
        } => release::codesign(&artifact, platform, config.as_deref()),
        Command::Notarize { artifact, config } => release::notarize(&artifact, config.as_deref()),
        Command::MakeUpdate {
            archive,
            url,
            version,
            target,
            format,
            key_env,
            output_dir,
        } => release::make_update(
            &archive,
            &url,
            &version,
            &target,
            format.unwrap_or_else(|| release::default_update_format(&target)),
            &key_env,
            &output_dir,
        ),
        Command::Clean { path } => clean::clean_project(&path),
        Command::Doctor => {
            doctor();
            Ok(())
        }
        Command::Logs {
            kind,
            tail,
            follow,
            path,
        } => logs::print_logs(kind, tail, follow, path),
        Command::Info => {
            info();
            Ok(())
        }
    }
}

pub fn init_project(path: &Path, name: Option<&str>) -> Result<()> {
    if path.exists() {
        anyhow::bail!("refusing to initialize existing path: {}", path.display());
    }

    let display_name = name.map_or_else(|| default_display_name(path), str::to_owned);
    let project_name = project_name_slug(&display_name);
    let identifier = format!("dev.cefari.{project_name}");

    fs::create_dir_all(path.join("frontend"))
        .with_context(|| format!("failed to create frontend directory at {}", path.display()))?;
    fs::create_dir_all(path.join("daemon"))
        .with_context(|| format!("failed to create daemon directory at {}", path.display()))?;

    write_file(
        &path.join("cefari.toml"),
        &initial_project_manifest(&project_name, &display_name, &identifier)?,
    )?;
    write_file(&path.join("frontend/index.html"), FRONTEND_TEMPLATE)?;
    write_file(&path.join("daemon/main.ts"), DAEMON_TEMPLATE)?;
    write_cefari_skill(path)?;
    write_file(
        &path.join("README.md"),
        &format!(
            r"# {display_name}

Generated by `cefari init`.

```bash
cefari dev
cefari build
cefari package
```
"
        ),
    )?;

    println!("created Cefari project at {}", path.display());
    Ok(())
}

fn initial_project_manifest(
    project_name: &str,
    display_name: &str,
    identifier: &str,
) -> Result<String> {
    toml::to_string_pretty(&InitialProjectManifest {
        app: InitialProjectApp {
            project_name,
            name: display_name,
            identifier,
        },
        frontend: InitialFrontendConfig {
            dist: "frontend/dist",
            dev_port: 5173,
        },
        daemon: InitialDaemonConfig {
            entry: "daemon/main.ts",
        },
        package: InitialPackageConfig {
            product_name: display_name,
        },
    })
    .context("failed to encode project manifest")
}

#[derive(Serialize)]
struct InitialProjectManifest<'a> {
    app: InitialProjectApp<'a>,
    frontend: InitialFrontendConfig<'a>,
    daemon: InitialDaemonConfig<'a>,
    package: InitialPackageConfig<'a>,
}

#[derive(Serialize)]
struct InitialProjectApp<'a> {
    project_name: &'a str,
    name: &'a str,
    identifier: &'a str,
}

#[derive(Serialize)]
struct InitialFrontendConfig<'a> {
    dist: &'a str,
    dev_port: u16,
}

#[derive(Serialize)]
struct InitialDaemonConfig<'a> {
    entry: &'a str,
}

#[derive(Serialize)]
struct InitialPackageConfig<'a> {
    product_name: &'a str,
}

fn doctor() {
    println!("Cefari doctor");
    print_tool_status("cargo");
    print_tool_status("deno");
    print_tool_status("cargo-packager");
    print_tool_status("cargo-codesign");
}

fn info() {
    println!("cefari-cli {}", env!("CARGO_PKG_VERSION"));
    println!("target os: {}", std::env::consts::OS);
    println!("target arch: {}", std::env::consts::ARCH);

    match ProjectConfig::load_from_dir(".") {
        Ok(project) => {
            println!("project: {}", project.app.name);
            println!("identifier: {}", project.app.identifier);
        }
        Err(project::LoadProjectError::Missing { .. }) => {
            println!("project: not found");
        }
        Err(error) => {
            println!("project: invalid ({error})");
        }
    }
}

fn print_tool_status(tool: &str) {
    if tool_available(tool) {
        println!("{tool}: found");
    } else {
        println!("{tool}: missing");
    }
}

pub(crate) fn tool_available(tool: &str) -> bool {
    ProcessCommand::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

pub(crate) fn run_process(command: &mut ProcessCommand, description: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run {description}"))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("{description} failed with status {status}");
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory at {}", parent.display()))?;
    }

    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn write_cefari_skill(project_dir: &Path) -> Result<()> {
    for (relative_path, contents) in CEFARI_SKILL_FILES {
        write_file(
            &project_dir
                .join(".agents/skills/cefari")
                .join(relative_path),
            contents,
        )?;
    }

    Ok(())
}

fn default_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("cefari-app")
        .to_owned()
}

fn identifier_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_owned()
}

fn project_name_slug(value: &str) -> String {
    let slug = identifier_slug(value);

    if slug.is_empty() {
        "cefari-app".to_owned()
    } else {
        slug
    }
}

const FRONTEND_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Cefari App</title>
  </head>
  <body>
    <main id="app"></main>
  </body>
</html>
"#;

const DAEMON_TEMPLATE: &str = r#"console.log("cefari daemon starting");
"#;

const CEFARI_SKILL_FILES: &[(&str, &str)] = &[
    ("SKILL.md", include_str!("../../../skills/cefari/SKILL.md")),
    (
        "agents/openai.yaml",
        include_str!("../../../skills/cefari/agents/openai.yaml"),
    ),
    (
        "references/project-creation.md",
        include_str!("../../../skills/cefari/references/project-creation.md"),
    ),
    (
        "references/template-authoring.md",
        include_str!("../../../skills/cefari/references/template-authoring.md"),
    ),
    (
        "references/release-workflows.md",
        include_str!("../../../skills/cefari/references/release-workflows.md"),
    ),
    (
        "references/packaging.md",
        include_str!("../../../skills/cefari/references/packaging.md"),
    ),
    (
        "references/daemon-behavior.md",
        include_str!("../../../skills/cefari/references/daemon-behavior.md"),
    ),
    (
        "references/troubleshooting.md",
        include_str!("../../../skills/cefari/references/troubleshooting.md"),
    ),
];

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command, identifier_slug, initial_project_manifest, project_name_slug};
    use crate::project::ProjectConfig;

    #[test]
    fn parses_planned_commands() {
        assert!(matches!(
            Cli::try_parse_from(["cefari", "init", "sample"])
                .expect("init should parse")
                .command,
            Command::Init { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from([
                "cefari",
                "dev",
                "sample",
                "--frontend-port",
                "0",
                "--devtools-port",
                "9333",
            ])
            .expect("dev should parse")
            .command,
            Command::Dev {
                frontend_port: Some(0),
                devtools_port: Some(9333),
                ..
            }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "doctor"])
                .expect("doctor should parse")
                .command,
            Command::Doctor
        ));
        assert!(matches!(
            Cli::try_parse_from([
                "cefari",
                "make-update",
                "release.tar.gz",
                "--url",
                "https://downloads.example.test/release.tar.gz",
                "--version",
                "1.2.3"
            ])
            .expect("make-update should parse")
            .command,
            Command::MakeUpdate { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "codesign", "Example.dmg"])
                .expect("codesign should parse")
                .command,
            Command::Codesign { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "notarize", "Example.dmg"])
                .expect("notarize should parse")
                .command,
            Command::Notarize { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "build", "sample"])
                .expect("build should parse")
                .command,
            Command::Build { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "build", "sample", "--release"])
                .expect("release build should parse")
                .command,
            Command::Build { release: true, .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "package", "sample"])
                .expect("package should parse")
                .command,
            Command::Package { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "package", "sample", "--release"])
                .expect("release package should parse")
                .command,
            Command::Package { release: true, .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["cefari", "clean", "sample"])
                .expect("clean should parse")
                .command,
            Command::Clean { .. }
        ));
    }

    #[test]
    fn initial_project_manifest_escapes_toml_strings() {
        let manifest = initial_project_manifest(
            "quoted-app",
            "Quoted \"App\" \\ Demo\nNext",
            "dev.cefari.quoted-app",
        )
        .expect("manifest should serialize");

        let project: ProjectConfig = toml::from_str(&manifest).expect("manifest should parse");

        assert_eq!(project.app.project_name, "quoted-app");
        assert_eq!(project.app.name, "Quoted \"App\" \\ Demo\nNext");
        assert_eq!(project.package.product_name, "Quoted \"App\" \\ Demo\nNext");
    }

    #[test]
    fn creates_identifier_slug() {
        assert_eq!(identifier_slug("Example App"), "example-app");
        assert_eq!(identifier_slug("  CEFARI__Desktop!! "), "cefari-desktop");
    }

    #[test]
    fn creates_project_name_slug() {
        assert_eq!(project_name_slug("Example App"), "example-app");
        assert_eq!(project_name_slug("!!!"), "cefari-app");
    }
}
