use anyhow::Result;
use cefari_core::{AppIdentity, RuntimePaths};

fn main() -> Result<()> {
    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    println!("cefari-desktop startup skeleton");
    println!("config: {}", paths.config_file.display());
    Ok(())
}
