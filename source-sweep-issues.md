# Source Sweep Issues

Full source sweep report for sprint `full-source-sweep`.

## Baseline

- Base: detached HEAD `63d288aa3d43a1e1319d121573392cd2daf2f0b0` (`63d288a`)
- Sprint branch: `sprint/full-source-sweep/review`
- Initial repo state: clean working tree, 151 tracked files
- Sweep scope: Rust crates, Deno TypeScript package, Vite React template, scripts, documentation, mirrored Cefari skill files, GitHub actions/workflows, generated TypeScript IPC bindings
- Exclusions: third-party dependencies, build outputs, VCS internals, and generated cache/output directories such as `target`, `node_modules`, `dist`, and `build`

## Finding Schema

Each finding uses:

- `ID`: stable source-sweep identifier
- `Severity`: `Critical`, `High`, `Medium`, `Low`, or `Info`
- `Status`: `Confirmed`, `Risk`, `Test Gap`, or `Documentation Drift`
- `Confidence`: `High`, `Medium`, or `Low`
- `Area`: affected source area
- `Files`: file references
- `Evidence`: source or command evidence
- `Impact`: potential user, developer, release, security, or maintenance impact
- `Suggested next step`: focused follow-up action
- `Verification notes`: commands, reproduction, or remaining manual verification

## Planned Validation Commands

| Command | Purpose | Result |
| --- | --- | --- |
| `cargo fmt --all --check` | Rust formatting | Pending |
| `cargo clippy --workspace --all-targets -- -D warnings` | Rust linting | Pending |
| `cargo test --workspace` | Rust test suite | Pending |
| `cargo test -p cefari-core` | Core crate tests | Passed |
| `cargo test -p cefari-desktop` | Desktop crate tests | Passed |
| `cargo test -p cefari-cli` | CLI crate tests | Failed |
| `deno task --cwd packages/cefari-app check` | TypeScript package type check | Pending |
| `deno task --cwd packages/cefari-app test` | TypeScript package tests | Pending |
| `deno task --cwd templates/vite-react-basic/frontend check` | Template frontend type check | Pending |
| `deno task --cwd templates/vite-react-basic/frontend build` | Template frontend build | Pending |
| `actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/*.yml docs/examples/cefari-release-workflow.yml` | Workflow syntax | Pending |
| `shellcheck scripts/extract-native-package-payload.sh .github/actions/cefari-release/release.sh` | Shell script diagnostics | Pending |
| `ruby -c scripts/sync-cefari-skill-docs.rb` | Ruby syntax check | Pending |
| `ruby -c scripts/verify-native-package-payload.rb` | Ruby syntax check | Pending |

## Findings

### SS-001: Default desktop builds cannot start the UI because CEF is disabled

- `Severity`: High
- `Status`: Confirmed
- `Confidence`: High
- `Area`: Desktop runtime, CLI build/package path
- `Files`: `crates/cefari-desktop/Cargo.toml`, `crates/cefari-desktop/src/desktop_cef.rs`, `crates/cefari-desktop/src/main.rs`, `crates/cefari-cli/src/build.rs`
- `Evidence`: `crates/cefari-desktop/Cargo.toml` sets `default = []` and makes `cef` optional. In the non-CEF implementation, `CefRuntime::create_browser` always returns `CEF feature disabled; rebuild cefari-desktop with --features cef`. Startup unconditionally calls `guards.cef_runtime.create_browser(&window, &shell_ui.url())` in `crates/cefari-desktop/src/main.rs`. The CLI `build_desktop` command runs `cargo build --manifest-path <workspace> -p cefari-desktop` without `--features cef`.
- `Impact`: `cefari build` can produce a desktop executable that fails before showing the app UI, even though the CEF-enabled source path compiles.
- `Suggested next step`: Decide whether the desktop crate should enable `cef` by default, whether CLI builds should pass `--features cef`, or whether no-CEF builds should be explicitly dev/test-only and never packaged.
- `Verification notes`: `cargo test -p cefari-desktop` passed for the default feature set. `cargo check -p cefari-desktop --features cef` passed, which confirms the CEF-enabled path currently compiles.

### SS-002: The desktop bridge is not wired into the CEF browser runtime

- `Severity`: High
- `Status`: Confirmed
- `Confidence`: High
- `Area`: Desktop runtime, TypeScript bridge
- `Files`: `crates/cefari-desktop/src/desktop_bridge.rs`, `crates/cefari-desktop/src/desktop_cef.rs`, `crates/cefari-desktop/src/main.rs`, `packages/cefari-app/src/transport.ts`
- `Evidence`: `desktop_bridge.rs` defines `CEFARI_BRIDGE_SCRIPT`, `BridgeOriginPolicy`, and `CefariBridge::handle_json_request`, but repository search found all references to `CefariBridge`, `BridgeOriginPolicy`, `CEFARI_BRIDGE_SCRIPT`, `__CEFARI_IPC_POST__`, and `__CEFARI_IPC_EVENT__` only in `desktop_bridge.rs` tests and TypeScript consumers. `desktop_cef.rs` calls `cef::browser_host_create_browser(..., None, ...)` with no CEF client or handler that injects the bridge script or forwards JavaScript messages to `DesktopIpcDispatcher`.
- `Impact`: App code using `@cefari/app` cannot receive a real `window.cefari` transport in the shipped desktop shell, so documented window, shell, service, tray, update, files, and notification APIs are unavailable at runtime.
- `Suggested next step`: Add CEF integration for script injection and request/event transport, then cover it with an integration or lower-level unit test that proves the bridge is connected outside `desktop_bridge.rs` tests.
- `Verification notes`: `rg` found no production references wiring the bridge into the CEF runtime. `cargo test -p cefari-desktop` passed because tests cover the isolated bridge dispatcher, not browser integration.

### SS-003: The CLI crate does not compile because a bundled skill reference is missing

- `Severity`: High
- `Status`: Confirmed
- `Confidence`: High
- `Area`: CLI, project scaffolding, test/build health
- `Files`: `crates/cefari-cli/src/lib.rs`, `skills/cefari/references/`, `templates/vite-react-basic/.agents/skills/cefari/references/template-authoring.md`
- `Evidence`: `crates/cefari-cli/src/lib.rs` includes `../../../skills/cefari/references/template-authoring.md` in `CEFARI_SKILL_FILES`, but `skills/cefari/references/` does not contain that file. The file exists only under `templates/vite-react-basic/.agents/skills/cefari/references/template-authoring.md`.
- `Impact`: `cargo test -p cefari-cli`, `cargo run -p cefari-cli`, and any workspace validation that compiles `cefari-cli` fail before tests or commands can run. The `cefari init` scaffolder also cannot embed the intended complete skill bundle from the root skill source.
- `Suggested next step`: Restore `skills/cefari/references/template-authoring.md` or remove the include and adjust scaffold expectations, then rerun the CLI and workspace test suites.
- `Verification notes`: `cargo test -p cefari-cli` failed with `couldn't read crates/cefari-cli/src/../../../skills/cefari/references/template-authoring.md`.

### SS-004: CLI-generated TOML and JSON can be invalid for names or manifest fields containing special characters

- `Severity`: Medium
- `Status`: Risk
- `Confidence`: High
- `Area`: CLI scaffolding and package metadata
- `Files`: `crates/cefari-cli/src/lib.rs`, `crates/cefari-cli/src/package.rs`, `crates/cefari-cli/src/cef.rs`
- `Evidence`: `init_project` writes `cefari.toml` with `format!` interpolation for `name = "{display_name}"` and `product_name = "{display_name}"` without TOML escaping. `PackageManifest::to_json` and `CefPreparationManifest::to_json` build JSON strings manually; `PackageManifest::to_json` interpolates `product_name`, `identifier`, and paths without JSON escaping. Tests only cover simple names and then parse generated JSON in the simple case.
- `Impact`: A valid user-provided display/product name or path containing quotes, backslashes, newlines, or other JSON/TOML-significant characters can generate invalid project manifests or package manifests, breaking `cefari init`, `cefari package`, release automation, or downstream tooling that parses `dist/package/manifest.json`.
- `Suggested next step`: Use `toml::to_string_pretty` or a typed serializable struct for `cefari.toml`, and use `serde_json` for all JSON manifest generation.
- `Verification notes`: Source-confirmed by manual string construction. A targeted reproduction could not run through the CLI because SS-003 currently prevents compiling `cefari-cli`.

## Reviewed Areas With No Findings

- `crates/cefari-core/src/config.rs`: config serialization, defaults, unknown-field rejection, and save/load error mapping reviewed with no findings.
- `crates/cefari-core/src/ipc.rs` and `crates/cefari-core/bindings/ipc.ts`: IPC command/result/event contracts and generated TypeScript binding currency reviewed with no findings.
- `crates/cefari-core/src/logging.rs`: runtime log config and rotated log pruning reviewed with no findings.
- `crates/cefari-core/src/paths.rs`: platform project directory resolution reviewed with no findings.
- `crates/cefari-core/src/resources.rs`: packaged resource path validation and existence checks reviewed with no findings.
- `crates/cefari-core/src/services.rs`: service-manager wrappers, default levels, Windows `sc.exe` status fallback, and tests reviewed with no findings.
- `crates/cefari-core/src/updates.rs`: updater configuration preparation, unconfigured-state handling, and update result modeling reviewed with no findings.
- `crates/cefari-desktop/src/desktop_files.rs`: app-data filesystem path validation, read/write, readdir, mkdir, rm, rename, copy, stat, access, and JavaScript number conversion reviewed with no additional findings.
- `crates/cefari-desktop/src/desktop_ipc.rs`: typed dispatcher behavior and unsupported command mapping reviewed with no additional findings beyond the bridge integration gap in SS-002.
- `crates/cefari-desktop/src/desktop_menu.rs` and `crates/cefari-desktop/src/desktop_tray.rs`: menu/tray command mapping reviewed with no findings.
- `crates/cefari-desktop/src/desktop_notifications.rs`: native notification setup/request modeling reviewed with no findings; notification IPC remains intentionally unsupported in current docs.
- `crates/cefari-desktop/src/desktop_ui.rs` and `crates/cefari-desktop/src/external.rs`: resource loading fallback, diagnostic view escaping, and external URL scheme filtering reviewed with no additional findings.
- `crates/cefari-cli/src/project.rs`: project manifest parsing, required sections, default frontend port, and `project_name` validation reviewed with no additional findings.
- `crates/cefari-cli/src/dev.rs`: dev process orchestration, static frontend server, daemon logging setup, and shutdown behavior reviewed with no findings beyond the previously recorded desktop CEF/bridge issues.
- `crates/cefari-cli/src/build.rs`: frontend/daemon/desktop artifact assembly reviewed with no additional findings beyond SS-001.
- `crates/cefari-cli/src/package.rs`, `crates/cefari-cli/src/cef.rs`, and `crates/cefari-cli/src/release.rs`: packaging, CEF preparation, signing, notarization, and update metadata flows reviewed with no additional findings beyond SS-004.
- `crates/cefari-cli/src/logs.rs` and `crates/cefari-cli/src/clean.rs`: log reading/following and generated artifact cleanup reviewed with no findings.

## Validation Summary

- `cargo test -p cefari-core`: passed. This ran 31 unit tests and doc tests successfully; the `native_service_lifecycle_smoke` integration test remains ignored by design because it installs and starts a native OS service.
- `cargo test -p cefari-desktop`: passed. This ran 30 unit tests successfully.
- `cargo check -p cefari-desktop --features cef`: passed.
- `cargo test -p cefari-cli`: failed before running tests because `skills/cefari/references/template-authoring.md` is missing while `crates/cefari-cli/src/lib.rs` includes it.

## Skipped Or Limited Checks

- `crates/cefari-core/tests/service_lifecycle.rs::native_service_lifecycle_smoke` was not run because it is explicitly ignored and requires a disposable host for native service installation/start/stop verification.
