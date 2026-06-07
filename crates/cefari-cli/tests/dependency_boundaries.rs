use std::{collections::BTreeSet, fs, path::Path};

use toml::Value;

const DESKTOP_ONLY_DEPENDENCIES: &[&str] = &[
    "cef",
    "muda",
    "open",
    "raw-window-handle",
    "single-instance",
    "tao",
    "tray-icon",
];

#[test]
fn desktop_only_dependencies_stay_out_of_core_and_cli() {
    let workspace = workspace_dir();

    for manifest in [
        workspace.join("crates/cefari-core/Cargo.toml"),
        workspace.join("crates/cefari-cli/Cargo.toml"),
    ] {
        let dependencies = dependency_names(&manifest);
        for dependency in DESKTOP_ONLY_DEPENDENCIES {
            assert!(
                !dependencies.contains(*dependency),
                "{} must not depend on desktop-only crate {dependency}",
                manifest.display()
            );
        }
    }
}

#[test]
fn desktop_crate_owns_native_shell_dependencies() {
    let manifest = workspace_dir().join("crates/cefari-desktop/Cargo.toml");
    let dependencies = dependency_names(&manifest);

    for dependency in DESKTOP_ONLY_DEPENDENCIES {
        assert!(
            dependencies.contains(*dependency),
            "{} should declare desktop dependency {dependency}",
            manifest.display()
        );
    }
}

fn workspace_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("cefari-cli should live under crates/cefari-cli")
        .to_path_buf()
}

fn dependency_names(manifest: &Path) -> BTreeSet<String> {
    let contents = fs::read_to_string(manifest)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest.display()));
    let manifest: Value = toml::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", manifest.display()));

    manifest
        .get("dependencies")
        .and_then(Value::as_table)
        .expect("manifest should have a dependencies table")
        .keys()
        .cloned()
        .collect()
}
