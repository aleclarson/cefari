use std::{path::Path, process::Command};

use anyhow::{Context, Result};

use crate::{run_process, tool_available};

pub(super) fn run_cargo_packager_if_available(package_dir: &Path) -> Result<()> {
    let use_cargo_subcommand = !tool_available("cargo-packager") && tool_available("cargo");
    if !tool_available("cargo-packager") && !use_cargo_subcommand {
        println!("cargo-packager not found; skipped native package invocation");
        return Ok(());
    }

    let package_dir = package_dir.canonicalize().with_context(|| {
        format!(
            "failed to resolve package directory at {}",
            package_dir.display()
        )
    })?;
    let config = package_dir.join("cargo-packager.toml");
    let output_dir = package_dir.join("output");
    let mut command = if use_cargo_subcommand {
        let mut command = Command::new("cargo");
        command.arg("packager");
        command
    } else {
        Command::new("cargo-packager")
    };
    command
        .arg("--config")
        .arg(&config)
        .arg("--out-dir")
        .arg(&output_dir);

    run_process(&mut command, "cargo-packager")?;
    println!("created native packages at {}", output_dir.display());
    Ok(())
}
