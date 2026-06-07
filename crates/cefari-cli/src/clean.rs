use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::project::ProjectConfig;

pub fn clean_project(project_dir: &Path) -> Result<()> {
    ProjectConfig::load_from_dir(project_dir)?;

    remove_generated_dir(&ProjectConfig::build_dir(project_dir))?;
    remove_generated_dir(&ProjectConfig::dist_dir(project_dir))?;

    println!("cleaned Cefari project at {}", project_dir.display());
    Ok(())
}

fn remove_generated_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| {
            format!("failed to remove generated directory at {}", path.display())
        })?;
    }

    Ok(())
}
