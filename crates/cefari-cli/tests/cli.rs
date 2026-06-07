use std::{
    fs,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

fn cefari() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cefari"))
}

#[test]
fn help_shows_planned_commands() {
    let output = cefari().arg("--help").output().expect("cefari should run");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("init"));
    assert!(stdout.contains("make-update"));
    assert!(stdout.contains("doctor"));
}

#[test]
fn init_creates_project_scaffold() {
    let root = temp_project_path();
    let output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Example App")
        .output()
        .expect("cefari init should run");

    assert_success(&output);
    assert!(root.join("cefari.toml").exists());
    assert!(root.join("frontend/index.html").exists());
    assert!(root.join("daemon/main.ts").exists());
    assert!(root.join("README.md").exists());

    let manifest = fs::read_to_string(root.join("cefari.toml")).expect("manifest should exist");
    assert!(manifest.contains(r#"name = "Example App""#));
    assert!(manifest.contains(r#"identifier = "dev.cefari.example-app""#));

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn info_reports_project_when_manifest_exists() {
    let root = temp_project_path();
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Info App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let output = cefari()
        .arg("info")
        .current_dir(&root)
        .output()
        .expect("cefari info should run");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("project: Info App"));
    assert!(stdout.contains("identifier: dev.cefari.info-app"));

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn build_creates_project_artifacts() {
    let root = temp_project_path();
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Build App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let output = cefari()
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");

    assert_success(&output);
    assert!(root.join("build/frontend/index.html").exists());
    assert!(root.join("build/daemon/main.ts").exists());
    assert!(root.join("frontend/dist/index.html").exists());

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn package_creates_assembly_manifest_after_build() {
    let root = temp_project_path();
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Package App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let build_output = cefari()
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");
    assert_success(&build_output);

    let output = cefari()
        .arg("package")
        .arg(&root)
        .output()
        .expect("cefari package should run");

    assert_success(&output);
    assert!(root.join("dist/package/cargo-packager.toml").exists());
    assert!(root.join("dist/package/manifest.json").exists());

    let manifest =
        fs::read_to_string(root.join("dist/package/manifest.json")).expect("manifest should exist");
    assert!(manifest.contains(r#""desktop_binary": "cefari-desktop""#));
    assert!(manifest.contains(r#""cef_resources": "pending-cef-download""#));

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn package_requires_build_artifacts() {
    let root = temp_project_path();
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Unbuilt App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let output = cefari()
        .arg("package")
        .arg(&root)
        .output()
        .expect("cefari package should run");

    assert!(!output.status.success());
    assert!(stderr(&output).contains("run cefari build first"));

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn doctor_reports_tool_statuses() {
    let output = cefari()
        .arg("doctor")
        .output()
        .expect("cefari doctor should run");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("Cefari doctor"));
    assert!(stdout.contains("cargo:"));
    assert!(stdout.contains("cargo-packager:"));
}

#[test]
fn unimplemented_command_fails_clearly() {
    let output = cefari().arg("dev").output().expect("cefari dev should run");

    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("cefari dev is not implemented yet"));
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn temp_project_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cefari-cli-integration-test-{suffix}-{count}"))
}
