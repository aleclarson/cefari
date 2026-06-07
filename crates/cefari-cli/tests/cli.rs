use std::{
    fs,
    path::{Path, PathBuf},
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
fn version_reports_package_version() {
    let output = cefari()
        .arg("--version")
        .output()
        .expect("cefari should run");

    assert_success(&output);
    assert_eq!(
        stdout(&output).trim(),
        format!("cefari {}", env!("CARGO_PKG_VERSION"))
    );
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

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let output = with_fake_cef_resources(cefari(), &cef_fixture)
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");

    assert_success(&output);
    assert!(root.join("build/frontend/index.html").exists());
    assert!(root.join("build/daemon/main.ts").exists());
    assert!(
        root.join("build/daemon")
            .join(daemon_executable_name())
            .exists()
    );
    assert!(root.join("build/cef/resources").exists());
    assert!(root.join("build/cef/manifest.json").exists());
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

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let build_output = with_fake_cef_resources(cefari(), &cef_fixture)
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
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest).expect("manifest should be valid JSON");
    assert!(manifest.contains(r#""desktop_binary": "cefari-desktop""#));
    assert!(manifest.contains(r#""daemon_executable": ""#));
    assert!(manifest.contains(daemon_executable_name()));
    assert!(manifest.contains(r#""cef_resources": ""#));
    assert!(manifest.contains(r#""cef_archive_json": ""#));
    assert!(manifest.contains("build/cef/resources"));
    assert!(manifest.contains("build/cef/resources/archive.json"));
    assert!(
        PathBuf::from(json_field(&manifest_json, "frontend_dir"))
            .join("index.html")
            .exists()
    );
    assert!(PathBuf::from(json_field(&manifest_json, "daemon_executable")).exists());
    assert!(PathBuf::from(json_field(&manifest_json, "cef_archive_json")).exists());
    assert!(
        PathBuf::from(json_field(&manifest_json, "cef_resources"))
            .join("libcef.fixture")
            .exists()
    );

    let metadata = fs::read_to_string(root.join("dist/package/cargo-packager.toml"))
        .expect("package metadata should exist");
    assert!(metadata.contains(r#"name = "dev.cefari.package-app""#));
    assert!(metadata.contains("target/debug"));
    assert!(metadata.contains("build/cef/resources"));

    fs::remove_dir_all(root).expect("temp project should be removable");
}

#[test]
fn package_release_metadata_uses_release_desktop_binary() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    create_fake_tool(
        &tools,
        "cargo-packager",
        r#"echo "cargo-packager $@" >> "$CEFARI_TOOL_LOG""#,
    );
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Release Package App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let build_output = with_fake_cef_resources(cefari(), &cef_fixture)
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");
    assert_success(&build_output);

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("package")
        .arg(&root)
        .arg("--release")
        .output()
        .expect("cefari package should run");
    assert_success(&output);

    let metadata = fs::read_to_string(root.join("dist/package/cargo-packager.toml"))
        .expect("package metadata should exist");
    assert!(metadata.contains("target/release"));
    assert!(metadata.contains(r#"path = "cefari-desktop""#));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
}

#[test]
fn package_invokes_cargo_packager_when_available() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    create_fake_tool(
        &tools,
        "cargo-packager",
        r#"echo "cargo-packager $@" >> "$CEFARI_TOOL_LOG""#,
    );

    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Package Tool App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let build_output = with_fake_cef_resources(cefari(), &cef_fixture)
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");
    assert_success(&build_output);

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("package")
        .arg(&root)
        .output()
        .expect("cefari package should run");

    assert_success(&output);
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("cargo-packager --config"));
    assert!(tool_log.contains("cargo-packager.toml"));
    assert!(tool_log.contains("--out-dir"));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
}

#[test]
fn package_invokes_cargo_packager_subcommand_when_binary_is_unavailable() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    create_fake_tool(&tools, "cargo", r#"echo "cargo $@" >> "$CEFARI_TOOL_LOG""#);

    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Package Subcommand App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let build_output = with_fake_cef_resources(cefari(), &cef_fixture)
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");
    assert_success(&build_output);

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("package")
        .arg(&root)
        .output()
        .expect("cefari package should run");

    assert_success(&output);
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("cargo packager --config"));
    assert!(tool_log.contains("cargo-packager.toml"));
    assert!(tool_log.contains("--out-dir"));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
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
fn clean_removes_generated_artifacts() {
    let root = temp_project_path();
    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Clean App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let cef_fixture = create_fake_cef_resources(&root.join("cef-fixture"));
    let build_output = with_fake_cef_resources(cefari(), &cef_fixture)
        .arg("build")
        .arg(&root)
        .output()
        .expect("cefari build should run");
    assert_success(&build_output);

    let package_output = cefari()
        .arg("package")
        .arg(&root)
        .output()
        .expect("cefari package should run");
    assert_success(&package_output);

    assert!(root.join("build").exists());
    assert!(root.join("dist").exists());

    let clean_output = cefari()
        .arg("clean")
        .arg(&root)
        .output()
        .expect("cefari clean should run");
    assert_success(&clean_output);

    assert!(!root.join("build").exists());
    assert!(!root.join("dist").exists());

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
fn codesign_invokes_cargo_codesign_for_macos_artifact() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    let artifact = root.join("Example.dmg");
    fs::create_dir_all(&root).expect("temp project should be created");
    fs::write(&artifact, "dmg").expect("artifact should be created");
    create_fake_tool(
        &tools,
        "cargo-codesign",
        r#"echo "cargo-codesign $@" >> "$CEFARI_TOOL_LOG""#,
    );

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("codesign")
        .arg(&artifact)
        .arg("--platform")
        .arg("macos")
        .output()
        .expect("cefari codesign should run");

    assert_success(&output);
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("cargo-codesign codesign macos --dmg"));
    assert!(tool_log.contains("--skip-notarize"));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
}

#[test]
fn notarize_invokes_cargo_codesign_macos_flow() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    let artifact = root.join("Example.dmg");
    fs::create_dir_all(&root).expect("temp project should be created");
    fs::write(&artifact, "dmg").expect("artifact should be created");
    create_fake_tool(
        &tools,
        "cargo-codesign",
        r#"echo "cargo-codesign $@" >> "$CEFARI_TOOL_LOG""#,
    );

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("notarize")
        .arg(&artifact)
        .output()
        .expect("cefari notarize should run");

    assert_success(&output);
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("cargo-codesign codesign macos --dmg"));
    assert!(!tool_log.contains("--skip-notarize"));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
}

#[test]
fn make_update_signs_archive_and_writes_updater_manifest() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    let archive = root.join("Example.tar.gz");
    let output_dir = root.join("dist/update");
    fs::create_dir_all(&root).expect("temp project should be created");
    fs::write(&archive, "archive").expect("archive should be created");
    create_fake_tool(
        &tools,
        "cargo-codesign",
        r#"echo "cargo-codesign $@" >> "$CEFARI_TOOL_LOG"
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    shift
    echo "fake-signature" > "$1"
    exit 0
  fi
  shift
done
exit 1"#,
    );

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("make-update")
        .arg(&archive)
        .arg("--url")
        .arg("https://downloads.example.test/Example.tar.gz")
        .arg("--version")
        .arg("1.2.3")
        .arg("--target")
        .arg("darwin-aarch64")
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .expect("cefari make-update should run");

    assert_success(&output);
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("cargo-codesign codesign update --archive"));
    assert!(output_dir.join("Example.tar.gz.sig").exists());

    let manifest =
        fs::read_to_string(output_dir.join("update.json")).expect("manifest should exist");
    assert!(manifest.contains(r#""version": "1.2.3""#));
    assert!(manifest.contains(r#""darwin-aarch64""#));
    assert!(manifest.contains(r#""format": "app""#));
    assert!(manifest.contains(r#""signature": "fake-signature""#));
    assert!(manifest.contains(r#""url": "https://downloads.example.test/Example.tar.gz""#));
    let update: cargo_packager_updater::RemoteRelease =
        serde_json::from_str(&manifest).expect("updater should parse generated manifest");
    assert_eq!(update.version.to_string(), "1.2.3");
    assert_eq!(
        update
            .download_url("darwin-aarch64")
            .expect("target URL should resolve")
            .as_str(),
        "https://downloads.example.test/Example.tar.gz"
    );
    assert_eq!(
        update
            .signature("darwin-aarch64")
            .expect("target signature should resolve"),
        "fake-signature"
    );
    assert_eq!(
        update
            .format("darwin-aarch64")
            .expect("target format should resolve")
            .to_string(),
        "app"
    );

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
}

#[test]
fn dev_orchestrates_frontend_daemon_and_desktop() {
    let root = temp_project_path();
    let tools = temp_project_path();
    let log = tools.join("tool.log");
    create_fake_tool(
        &tools,
        "deno",
        r#"echo "deno $@" >> "$CEFARI_TOOL_LOG"
sleep 1
exit 0"#,
    );
    create_fake_tool(
        &tools,
        "cargo",
        r#"echo "cargo $@" >> "$CEFARI_TOOL_LOG"
sleep 5"#,
    );

    let init_output = cefari()
        .arg("init")
        .arg(&root)
        .arg("--name")
        .arg("Dev App")
        .output()
        .expect("cefari init should run");
    assert_success(&init_output);

    let output = with_fake_tools(cefari(), &tools, &log)
        .arg("dev")
        .arg(&root)
        .arg("--frontend-port")
        .arg("0")
        .output()
        .expect("cefari dev should run");

    assert_success(&output);
    let stdout = stdout(&output);
    assert!(stdout.contains("frontend dev server: http://"));
    let tool_log = fs::read_to_string(&log).expect("tool log should exist");
    assert!(tool_log.contains("deno run --watch --allow-read --allow-net daemon/main.ts"));
    assert!(tool_log.contains("cargo run --manifest-path"));
    assert!(tool_log.contains("cefari-desktop"));

    fs::remove_dir_all(root).expect("temp project should be removable");
    fs::remove_dir_all(tools).expect("temp tools should be removable");
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

fn json_field<'a>(value: &'a serde_json::Value, field: &str) -> &'a str {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("manifest field {field} should be a string"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn daemon_executable_name() -> &'static str {
    if cfg!(windows) {
        "cefari-daemon.exe"
    } else {
        "cefari-daemon"
    }
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

fn with_fake_tools(mut command: Command, tools_dir: &Path, log: &Path) -> Command {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![tools_dir.to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let path = std::env::join_paths(paths).expect("PATH should be joinable");
    command.env("PATH", path);
    command.env("CEFARI_TOOL_LOG", log);
    command
}

fn with_fake_cef_resources(mut command: Command, fixture: &Path) -> Command {
    command.env("CEFARI_CEF_RESOURCES_DIR", fixture);
    command
}

fn create_fake_cef_resources(path: &Path) -> PathBuf {
    fs::create_dir_all(path).expect("CEF fixture dir should be created");
    fs::write(
        path.join("archive.json"),
        r#"{
  "type": "minimal",
  "name": "cef_binary_148.0.10+gfixture+chromium-148.0.0_macosarm64_minimal.tar.bz2",
  "sha1": "fixture-sha1"
}"#,
    )
    .expect("CEF archive metadata should be written");
    fs::write(path.join("libcef.fixture"), "fixture").expect("CEF fixture file should be written");
    path.to_path_buf()
}

fn create_fake_tool(tools_dir: &Path, name: &str, body: &str) -> PathBuf {
    fs::create_dir_all(tools_dir).expect("tools dir should be created");
    let tool = tools_dir.join(name);
    fs::write(&tool, format!("#!/bin/sh\n{body}\n")).expect("fake tool should be written");
    make_executable(&tool);
    tool
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .expect("fake tool metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("fake tool should be executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
