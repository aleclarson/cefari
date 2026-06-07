use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cefari")]
#[command(about = "Create, develop, build, package, sign, and release Cefari apps.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new Cefari project.
    Init,
    /// Run the local development environment.
    Dev,
    /// Build frontend, daemon, and desktop artifacts.
    Build,
    /// Package a built Cefari app.
    Package,
    /// Code sign a packaged app.
    Codesign,
    /// Notarize a signed app.
    Notarize,
    /// Generate update artifacts.
    MakeUpdate,
    /// Check local tool and project health.
    Doctor,
    /// Print environment and project information.
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => todo_command("init"),
        Command::Dev => todo_command("dev"),
        Command::Build => todo_command("build"),
        Command::Package => todo_command("package"),
        Command::Codesign => todo_command("codesign"),
        Command::Notarize => todo_command("notarize"),
        Command::MakeUpdate => todo_command("make-update"),
        Command::Doctor => todo_command("doctor"),
        Command::Info => todo_command("info"),
    }
}

fn todo_command(name: &str) -> Result<()> {
    anyhow::bail!("cefari {name} is not implemented yet")
}
