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
| `cargo fmt --all --check` | Rust formatting | Passed |
| `cargo check --workspace` | Rust workspace compile check | Failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Rust linting | Failed |
| `cargo test --workspace` | Rust test suite | Failed |
| `cargo test -p cefari-core` | Core crate tests | Passed |
| `cargo test -p cefari-desktop` | Desktop crate tests | Passed |
| `npm run --prefix packages/cefari-cli test` | CLI crate tests | Failed |
| `deno task --cwd packages/cefari-app check` | TypeScript package type check | Passed |
| `deno task --cwd packages/cefari-app test` | TypeScript package tests | Passed |
| `deno task --cwd templates/vite-react-basic/frontend check` | Template frontend type check | Passed |
| `deno task --cwd templates/vite-react-basic/frontend build` | Template frontend build | Passed |
| `actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/*.yml docs/examples/cefari-release-workflow.yml` | Workflow syntax | Passed |
| `shellcheck scripts/extract-native-package-payload.sh .github/actions/cefari-release/release.sh` | Shell script diagnostics | Passed |
| `ruby -c scripts/sync-cefari-skill-docs.rb` | Ruby syntax check | Passed |
| `ruby -c scripts/verify-native-package-payload.rb` | Ruby syntax check | Passed |
| `scripts/sync-cefari-skill-docs.rb --check` | Skill docs mirror sync | Failed |

## Findings

### SS-001: Default desktop builds cannot start the UI because CEF is disabled

- `Severity`: High
- `Status`: Confirmed
- `Confidence`: High
- `Area`: Desktop runtime, CLI build/package path
- `Files`: `crates/cefari-desktop/Cargo.toml`, `crates/cefari-desktop/src/desktop_cef.rs`, `crates/cefari-desktop/src/main.rs`, `packages/cefari-cli/src/build.rs`
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
- `Files`: `packages/cefari-cli/src/lib.rs`, `skills/cefari/references/`, `templates/vite-react-basic/.agents/skills/cefari/references/template-authoring.md`
- `Evidence`: `packages/cefari-cli/src/lib.rs` includes `../../../skills/cefari/references/template-authoring.md` in `CEFARI_SKILL_FILES`, but `skills/cefari/references/` does not contain that file. The file exists only under `templates/vite-react-basic/.agents/skills/cefari/references/template-authoring.md`.
- `Impact`: `npm run --prefix packages/cefari-cli test`, `cargo run -p cefari-cli`, and any workspace validation that compiles `cefari-cli` fail before tests or commands can run. The `cefari init` scaffolder also cannot embed the intended complete skill bundle from the root skill source.
- `Suggested next step`: Restore `skills/cefari/references/template-authoring.md` or remove the include and adjust scaffold expectations, then rerun the CLI and workspace test suites.
- `Verification notes`: `npm run --prefix packages/cefari-cli test` failed with `couldn't read packages/cefari-cli/src/../../../skills/cefari/references/template-authoring.md`.

### SS-004: CLI-generated TOML and JSON can be invalid for names or manifest fields containing special characters

- `Severity`: Medium
- `Status`: Risk
- `Confidence`: High
- `Area`: CLI scaffolding and package metadata
- `Files`: `packages/cefari-cli/src/lib.rs`, `packages/cefari-cli/src/package.rs`, `packages/cefari-cli/src/cef.rs`
- `Evidence`: `init_project` writes `cefari.toml` with `format!` interpolation for `name = "{display_name}"` and `product_name = "{display_name}"` without TOML escaping. `PackageManifest::to_json` and `CefPreparationManifest::to_json` build JSON strings manually; `PackageManifest::to_json` interpolates `product_name`, `identifier`, and paths without JSON escaping. Tests only cover simple names and then parse generated JSON in the simple case.
- `Impact`: A valid user-provided display/product name or path containing quotes, backslashes, newlines, or other JSON/TOML-significant characters can generate invalid project manifests or package manifests, breaking `cefari init`, `cefari package`, release automation, or downstream tooling that parses `dist/package/manifest.json`.
- `Suggested next step`: Use `toml::to_string_pretty` or a typed serializable struct for `cefari.toml`, and use `serde_json` for all JSON manifest generation.
- `Verification notes`: Source-confirmed by manual string construction. A targeted reproduction could not run through the CLI because SS-003 currently prevents compiling `cefari-cli`.

### SS-005: The checked-in Cefari skill docs mirror is stale

- `Severity`: Medium
- `Status`: Documentation Drift
- `Confidence`: High
- `Area`: Skill docs, CLI documentation, automation
- `Files`: `docs/cli/index.md`, `docs/cli/diagnostics.md`, `skills/cefari/docs/cli/index.md`, `skills/cefari/docs/cli/diagnostics.md`, `scripts/sync-cefari-skill-docs.rb`
- `Evidence`: `scripts/sync-cefari-skill-docs.rb --check` failed and reported stale files under `skills/cefari/docs`. The root CLI docs list `cefari logs` in `docs/cli/index.md` and document the command, options, and log streams in `docs/cli/diagnostics.md`; the mirrored skill docs omit the `logs` entry and the entire `cefari logs` section.
- `Impact`: Agents or users relying on the bundled Cefari skill docs receive incomplete CLI guidance and may miss the supported log inspection workflow.
- `Suggested next step`: Run `scripts/sync-cefari-skill-docs.rb` after deciding whether the current root docs are authoritative, then commit the mirrored skill doc updates.
- `Verification notes`: `ruby -c scripts/sync-cefari-skill-docs.rb` passed. `scripts/sync-cefari-skill-docs.rb --check` failed specifically because `skills/cefari/docs/cli/diagnostics.md` and `skills/cefari/docs/cli/index.md` are stale.

### SS-006: Public docs describe the desktop bridge as runtime-installed before the CEF runtime wires it

- `Severity`: Medium
- `Status`: Documentation Drift
- `Confidence`: High
- `Area`: Documentation, template guidance, desktop runtime
- `Files`: `docs/ipc.md`, `docs/typescript/raw-ipc.md`, `docs/css-contract.md`, `templates/vite-react-basic/README.md`, `crates/cefari-desktop/src/desktop_bridge.rs`, `crates/cefari-desktop/src/desktop_cef.rs`
- `Evidence`: `docs/ipc.md` says the desktop bridge installs `window.cefari` for trusted origins and installs the default CSS contract. `docs/typescript/raw-ipc.md` says the desktop runtime installs `window.cefari` for trusted packaged and localhost origins. Template guidance also describes native bridge usage inside trusted Cefari pages. This conflicts with SS-002: source search found the bridge script and dispatcher integration only in isolated bridge tests and TypeScript consumers, while `desktop_cef.rs` creates the browser with no CEF client or handler to inject the bridge or forward JavaScript messages.
- `Impact`: Application developers can follow the docs and template guidance, write against `@cefari/app`, and still receive an unavailable bridge in the actual desktop shell.
- `Suggested next step`: Either wire the bridge into the CEF runtime and keep the docs as intended behavior, or revise the docs/template guidance to describe the feature as not yet implemented.
- `Verification notes`: Source review and `rg` reference checks support the same root runtime gap recorded in SS-002; this finding records the separate documentation and template drift.

### SS-007: Workspace clippy fails on the default desktop no-CEF implementation

- `Severity`: Low
- `Status`: Confirmed
- `Confidence`: High
- `Area`: Desktop runtime, CI lint health
- `Files`: `crates/cefari-desktop/src/desktop_cef.rs`
- `Evidence`: `cargo clippy --workspace --all-targets -- -D warnings` reported `clippy::unused_self` for the default-feature `CefRuntime::create_browser(&self, ...)` implementation in the non-CEF module. The method always bails and does not read `self`.
- `Impact`: A strict workspace clippy gate fails even after accounting for the CLI compile blocker in SS-003, so CI or local pre-merge validation cannot currently use the documented `-D warnings` lint command successfully.
- `Suggested next step`: Either make the non-CEF method use `self` meaningfully, change it to an associated function behind a matching abstraction, or add a narrow allow if keeping the method signature aligned with the CEF-enabled implementation is intentional.
- `Verification notes`: Observed during the final `cargo clippy --workspace --all-targets -- -D warnings` run. The same command also failed on SS-003.

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
- `packages/cefari-cli/src/project.rs`: project manifest parsing, required sections, default frontend port, and `project_name` validation reviewed with no additional findings.
- `packages/cefari-cli/src/dev.rs`: dev process orchestration, static frontend server, daemon logging setup, and shutdown behavior reviewed with no findings beyond the previously recorded desktop CEF/bridge issues.
- `packages/cefari-cli/src/build.rs`: frontend/daemon/desktop artifact assembly reviewed with no additional findings beyond SS-001.
- `packages/cefari-cli/src/package.rs`, `packages/cefari-cli/src/cef.rs`, and `packages/cefari-cli/src/release.rs`: packaging, CEF preparation, signing, notarization, and update metadata flows reviewed with no additional findings beyond SS-004.
- `packages/cefari-cli/src/logs.rs` and `packages/cefari-cli/src/clean.rs`: log reading/following and generated artifact cleanup reviewed with no findings.
- `packages/cefari-app/src/transport.ts`, `results.ts`, and namespace wrappers: bridge availability handling, typed result extraction, event filtering, errors, shell/window/update/service/tray/notification wrappers, and exported API shape reviewed with no additional findings beyond the runtime bridge issue in SS-002.
- `packages/cefari-app/src/fs.ts` and `files.ts`: file API encoding, JSON helpers, object URL creation, app-data access, and result mapping reviewed with no findings.
- `packages/cefari-app/tests/cefari_app_test.ts`: package tests cover unavailable bridge behavior, namespace command wrapping, typed event filters, and typed IPC failures; reviewed with no findings.
- `templates/vite-react-basic/`: Deno workspace config, Vite frontend, React app, daemon entrypoint, `cefari.toml`, and template workflows were reviewed with no additional findings beyond source/runtime issues already recorded.
- `.github/actions/cefari-release/action.yml` and `.github/actions/cefari-release/release.sh`: release action inputs, script validation, and command flow reviewed with no standalone findings beyond SS-001 and SS-003, which currently affect its `cefari build` and `cefari package` steps.
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/platform-verification.yml`, `templates/vite-react-basic/.github/workflows/*.yml`, and `docs/examples/cefari-release-workflow.yml`: workflow syntax and validation coverage reviewed with no additional findings beyond failures already captured by SS-003 and SS-005.
- `scripts/extract-native-package-payload.sh`, `scripts/sync-cefari-skill-docs.rb`, and `scripts/verify-native-package-payload.rb`: path handling, payload checks, and syntax reviewed with no script implementation findings. The sync script correctly reported the stale skill mirror in SS-005.
- `docs/` and `skills/cefari/`: documentation set reviewed against current source behavior with no additional findings beyond SS-005, SS-006, and source issues already recorded.

## Validation Summary

- `cargo fmt --all --check`: passed.
- `cargo check --workspace`: failed because `skills/cefari/references/template-authoring.md` is missing while `packages/cefari-cli/src/lib.rs` includes it.
- `cargo clippy --workspace --all-targets -- -D warnings`: failed because of SS-003 and because `crates/cefari-desktop/src/desktop_cef.rs` triggers `clippy::unused_self` in the default no-CEF implementation.
- `cargo test --workspace`: failed before running the full workspace suite because of SS-003.
- `cargo test -p cefari-core`: passed. This ran 31 unit tests and doc tests successfully; the `native_service_lifecycle_smoke` integration test remains ignored by design because it installs and starts a native OS service.
- `cargo test -p cefari-desktop`: passed. This ran 30 unit tests successfully.
- `cargo check -p cefari-desktop --features cef`: passed.
- `npm run --prefix packages/cefari-cli test`: failed before running tests because `skills/cefari/references/template-authoring.md` is missing while `packages/cefari-cli/src/lib.rs` includes it.
- `deno task --cwd packages/cefari-app check`: passed.
- `deno task --cwd packages/cefari-app test`: passed, 4 tests.
- `deno task --cwd templates/vite-react-basic/frontend check`: passed.
- `deno task --cwd templates/vite-react-basic/frontend build`: passed. Generated `templates/vite-react-basic/frontend/dist` and `templates/vite-react-basic/node_modules` were removed after validation to keep the worktree clean.
- `actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/*.yml docs/examples/cefari-release-workflow.yml`: passed.
- `shellcheck scripts/extract-native-package-payload.sh .github/actions/cefari-release/release.sh`: passed.
- `ruby -c scripts/sync-cefari-skill-docs.rb`: passed.
- `ruby -c scripts/verify-native-package-payload.rb`: passed.
- `scripts/sync-cefari-skill-docs.rb --check`: failed because `skills/cefari/docs/cli/diagnostics.md` and `skills/cefari/docs/cli/index.md` are stale relative to the root docs.

## Skipped Or Limited Checks

- `crates/cefari-core/tests/service_lifecycle.rs::native_service_lifecycle_smoke` was not run because it is explicitly ignored and requires a disposable host for native service installation/start/stop verification.
- End-to-end release workflows, signing, notarization, update publishing, and packaged application launch were not run because they require platform-specific runners, secrets, release artifacts, and a compiling CLI; SS-003 prevents the current CLI from compiling.
- A live desktop GUI launch was not run during this inventory sprint. Source and compile checks were used instead; SS-001 and SS-002 identify the main runtime launch risks found by inspection.

## Environment Constraints

- The sweep ran on macOS in the local Codex worktree at `/Users/alec/.codex/worktrees/4f1a/cefari`.
- Networked release, signing, notarization, package publishing, and native service lifecycle checks were treated as out of scope for local validation.
- Build output directories such as `target`, template `node_modules`, and template `dist` were excluded from source review.

## Open Questions

- Should no-CEF desktop builds be supported as a first-class runtime mode, or should all user-facing build/package paths require the `cef` feature?
- Should the documented `window.cefari` bridge behavior be treated as intended behavior to implement next, or should public docs be revised until CEF integration exists?
- Should the root docs or mirrored skill docs be considered authoritative when the sync check fails?
