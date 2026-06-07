# Cefari Implementation Todo

This task list is derived from [README.md](README.md). It treats the architecture plan as a sequence of larger implementation tracks, with child tasks nested under the work they belong to.

Nesting audit: 135 of the 182 checklist entries are already nested child tasks under larger work items. Of the remaining unfinished work, 2 unchecked checklist entries are dependent child tasks rather than standalone project goals, and they sit under 2 unfinished parent tasks. Completed historical children stay nested for traceability, and open decisions remain top-level because they can unblock multiple tracks. The current unfinished child-task breakdown is:

- 1 under release automation.
- 1 under desktop runtime behavior verification.

## 1. Establish The Workspace

- [x] Create the Rust workspace skeleton.
  - [x] Create the root `Cargo.toml`.
  - [x] Add workspace members for `crates/cefari-core`, `crates/cefari-desktop`, and `crates/cefari-cli`.
  - [x] Set shared workspace metadata: edition, license, repository, authors, rust-version, and package defaults.
  - [x] Decide and document the minimum supported Rust version.
- [x] Add baseline repository hygiene.
  - [x] Add `.gitignore` entries for Rust, build artifacts, generated packages, CEF downloads, logs, and local environment files.
  - [x] Add a formatting policy using `rustfmt`.
  - [x] Add a linting policy using `clippy`.
  - [x] Decide not to add a root `justfile`, `Makefile`, or script directory until repeated commands justify one.
- [x] Document the architecture boundary.
  - [x] Add a short note that restates the runtime versus developer tooling split.
  - [x] Call out that `cefari-core` and `cefari-desktop` are runtime code.
  - [x] Call out that `cefari-cli` is distributed separately as developer orchestration tooling.

## 2. Build `cefari-core`

- [x] Scaffold the reusable runtime library crate.
  - [x] Create `crates/cefari-core`.
  - [x] Pin and add runtime dependencies: `serde`, `serde_json`, `directories`, `tracing`, `anyhow`, `thiserror`, `service-manager`, `cargo-packager-updater`, and `cargo-packager-resource-resolver`.
  - [x] Define core error types with `thiserror`.
  - [x] Define public result aliases and error conversion boundaries.
- [x] Implement runtime path and config support.
  - [x] Resolve config, data, cache, log, resource, and update artifact paths.
  - [x] Add configuration schema structs.
  - [x] Add configuration loading and saving.
  - [x] Add JSON parsing and validation tests for configuration data.
- [x] Implement runtime resource support.
  - [x] Wrap `cargo-packager-resource-resolver`.
  - [x] Define missing-resource error behavior.
  - [x] Add host-independent tests where possible.
- [x] Implement runtime logging support.
  - [x] Define logging configuration consumed by `cefari-desktop`.
  - [x] Provide helpers for log file paths and tracing setup inputs.
  - [x] Document which logging setup remains desktop-owned.
- [x] Implement update support.
  - [x] Define update-check configuration and state types.
  - [x] Prepare `cargo-packager-updater` config from Cefari update settings.
  - [x] Implement update check helpers using `cargo-packager-updater`.
  - [x] Implement update install helpers with clear failure states.
  - [x] Add tests around update state and error mapping where possible.
- [x] Implement service management support.
  - [x] Define service operations for install, start, stop, restart, status, and uninstall.
  - [x] Implement service helpers using `service-manager`.
  - [x] Add tests for platform-independent service configuration behavior.
- [x] Document the `cefari-core` API surface.
  - [x] Add crate-level docs.
  - [x] Mark which APIs are stable for `cefari-desktop`.
  - [x] Keep internal helpers private unless a runtime caller needs them.

## 3. Build `cefari-desktop`

- [x] Scaffold the shipped desktop app crate.
  - [x] Create `crates/cefari-desktop`.
  - [x] Add a path dependency on `cefari-core`.
  - [x] Pin and add desktop dependencies: `tao`, optional `cef`, `raw-window-handle`, `single-instance`, `tracing-subscriber`, `tracing-appender`, `anyhow`, `muda`, `tray-icon`, and `open`.
- [x] Implement desktop startup.
  - [x] Initialize runtime logging through `cefari-core`.
  - [x] Implement single-instance locking.
  - [x] Define startup error reporting behavior before the UI is available.
- [x] Implement the native shell.
  - [x] Create the Tao event loop.
  - [x] Create the main application window.
  - [x] Initialize CEF in the desktop process.
  - [x] Provision CMake for local and CI `cefari-desktop --features cef` builds.
  - [x] Load packaged UI resources promptly at startup.
  - [x] Add a fallback or diagnostic view for missing UI resources.
- [x] Implement desktop integration.
  - [x] Wire native menus through `muda`.
  - [x] Wire tray or menu-bar icon behavior through `tray-icon`.
  - [x] Add external link and file opening helpers through `open`.
- [x] Integrate runtime operations.
  - [x] Wire update check and install flow through `cefari-core`.
  - [x] Wire daemon service install/start/stop/status behavior through `cefari-core`.
- [x] Guard desktop-only boundaries.
  - [x] Add smoke tests or compile-time checks for desktop-only dependency placement.
  - [x] Confirm Tao and CEF dependencies are not introduced into `cefari-core` or `cefari-cli`.

## 4. Build `cefari-cli`

- [x] Scaffold the developer-facing CLI crate.
  - [x] Create `crates/cefari-cli`.
  - [x] Set the binary name to `cefari`.
  - [x] Pin and add CLI dependencies: `clap`, `anyhow`, `xshell`, `duct`, `camino`, `serde`, and `toml`.
  - [x] Define the top-level CLI parser and command enum.
- [x] Implement project creation.
  - [x] Implement `cefari init`.
  - [x] Define the generated Cefari project layout.
  - [x] Add typed parsing for generated `cefari.toml` manifests.
  - [x] Add template files for frontend, daemon, desktop config, and package metadata.
  - [x] Add fixture-based tests for generated project scaffolds.
- [x] Implement development orchestration.
  - [x] Implement `cefari dev`.
  - [x] Run the frontend dev server.
  - [x] Run the Deno daemon.
  - [x] Run the Rust desktop app.
  - [x] Handle process shutdown and error reporting.
- [x] Implement build orchestration.
  - [x] Implement `cefari build`.
  - [x] Build frontend artifacts.
  - [x] Build Deno daemon artifacts.
  - [x] Compile the Deno daemon entry into a packaged daemon executable.
  - [x] Build the Rust desktop app.
  - [x] Implement CEF preparation manifest as a CLI-owned step.
  - [x] Implement CEF binary download/cache population as a CLI-owned step.
- [x] Implement packaging and release commands.
  - [x] Implement `cefari package` package assembly preparation.
  - [x] Invoke `cargo-packager` from `cefari package` when available.
  - [x] Implement `cefari codesign` to invoke `cargo-codesign`.
  - [x] Implement `cefari notarize` for platform notarization flow.
  - [x] Implement `cefari make-update` to generate update artifacts.
  - [x] Add clean and dist task support if needed by build/package workflows.
- [x] Implement diagnostics.
  - [x] Implement `cefari doctor`.
  - [x] Implement `cefari info`.
  - [x] Include generated project manifest details in `cefari info`.
  - [x] Report when external tools such as `cargo-packager` or `cargo-codesign` are missing.
- [x] Test CLI behavior.
  - [x] Add parser tests for planned commands.
  - [x] Add integration tests for command dispatch.
  - [x] Promote scaffold tests into integration tests once CLI code is split out of `main.rs`.
  - [x] Confirm `clap` and CLI-only orchestration dependencies are not introduced into runtime crates.

## 5. Create The Packaging And Release Pipeline

- [x] Define package assembly.
  - [x] Define package metadata consumed by `cargo-packager`.
  - [x] Define how generated frontend artifacts are copied into packaged resources.
  - [x] Define how generated Deno daemon artifacts are included in app packages.
  - [x] Define how prepared CEF resources are resolved during package creation.
  - [x] Define how downloaded CEF binaries are verified and included during package creation.
- [x] Add CI coverage.
  - [x] Add CI steps for formatting, linting, testing, and workspace builds.
  - [x] Add CI steps that install or provide `cargo-packager` and `cargo-codesign`.
  - [x] Add platform-specific package assembly jobs.
  - [x] Add platform-specific native installer packaging jobs once CEF binaries are included.
- [ ] Add release automation.
  - [x] Add signing and notarization jobs for supported platforms.
  - [x] Add update artifact generation and publishing jobs.
  - [x] Verify package assembly contains generated UI, daemon, CEF preparation metadata, and separate CLI output in CI.
  - [ ] Verify native release packages contain `cefari-desktop`, `cefari-core` runtime code, CEF binaries/resources, and generated app artifacts.
  - [x] Verify `cefari-cli` is built, versioned, and distributed separately from desktop app packages.

## 6. Keep Dependencies Honest

- [x] Pin implementation dependency versions.
  - [x] Replace every README placeholder dependency version with a real version.
  - [x] Use workspace dependency declarations where it improves consistency.
  - [x] Decide whether shared versions should be centralized in workspace dependencies.
- [x] Audit crate boundaries.
  - [x] Confirm runtime crates do not depend on developer orchestration crates.
  - [x] Confirm CLI-only crates do not pull in Tao or CEF.
  - [x] Add dependency review notes for native and packaging crates.
  - [x] Keep new dependencies out unless they directly serve the README architecture.

## 7. Maintain Documentation

- [x] Update project documentation after scaffolding.
  - [x] Update `README.md` to reflect actual commands and crate paths.
  - [x] Document runtime versus CLI responsibility boundaries in contributor-facing docs.
- [x] Document developer workflows.
  - [x] Add CLI usage documentation for each implemented `cefari` command.
  - [x] Add CLI usage documentation for `dev` once implemented.
  - [x] Add CLI usage documentation for signing, notarization, and update commands.
  - [x] Add development setup documentation for CEF preparation.
  - [x] Add troubleshooting documentation for common `cefari doctor` failures.
- [x] Document release workflows.
  - [x] Add packaging documentation.
  - [x] Add signing and notarization documentation.
  - [x] Add update release documentation.

## 8. Verification Milestones

- [x] Verify workspace health.
  - [x] `cargo fmt --all` passes.
  - [x] `cargo clippy --workspace --all-targets` passes.
  - [x] `cargo test --workspace` passes.
- [x] Verify CLI workflows.
  - [x] `cargo run -p cefari-cli -- --help` shows all planned commands.
  - [x] `cargo run -p cefari-cli -- init` creates a valid sample app.
  - [x] `cargo run -p cefari-cli -- build` creates frontend and daemon artifacts and builds `cefari-desktop`.
  - [x] `cargo run -p cefari-cli -- doctor` reports required tool availability.
- [ ] Verify desktop runtime behavior.
  - [x] `cargo run -p cefari-desktop` starts a window and initializes runtime logging.
  - [x] `cargo run -p cefari-desktop` initializes runtime logging and the single-instance lock.
  - [x] A development app can load UI resources through the desktop shell.
  - [x] Service operation wrappers dispatch install, start, status, stop, restart, and uninstall through `service-manager`.
  - [ ] Service management operations are verified on each supported platform.
- [x] Verify package and update behavior.
  - [x] A packaged app contains the expected runtime, CEF, UI, and daemon artifacts.
  - [x] An update artifact can be generated and consumed by the runtime update flow.

## 9. Open Decisions

- [x] Decide supported operating systems and platform priority.
- [x] Decide the frontend stack expected by `cefari init`.
- [x] Decide the initial Deno daemon build output contract.
- [x] Decide the final Deno daemon build output contract for packaged releases.
- [x] Decide the initial Deno daemon project shape.
- [x] Decide CEF version pinning.
- [x] Decide CEF download source.
- [x] Decide when to make the `cef` dependency non-optional in `cefari-desktop`.
- [x] Decide package identifiers, signing identities, and notarization requirements.
- [x] Decide update server/artifact hosting expectations.
- [x] Decide whether `cefari-cli` should support plugins or project hooks.
- [x] Decide compatibility promises for generated project templates.
