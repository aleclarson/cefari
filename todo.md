# Cefari Implementation Todo

This task list is derived from [README.md](README.md). It treats the architecture plan as a sequence of larger implementation tracks, with child tasks nested under the work they belong to.

## 1. Establish The Workspace

- [ ] Create the Rust workspace skeleton.
  - [ ] Create the root `Cargo.toml`.
  - [ ] Add workspace members for `crates/cefari-core`, `crates/cefari-desktop`, and `crates/cefari-cli`.
  - [ ] Set shared workspace metadata: edition, license, repository, authors, rust-version, and package defaults.
  - [ ] Decide and document the minimum supported Rust version.
- [ ] Add baseline repository hygiene.
  - [ ] Add `.gitignore` entries for Rust, build artifacts, generated packages, CEF downloads, logs, and local environment files.
  - [ ] Add a formatting policy using `rustfmt`.
  - [ ] Add a linting policy using `clippy`.
  - [ ] Add a root `justfile`, `Makefile`, or script directory only if it matches the intended developer workflow.
- [ ] Document the architecture boundary.
  - [ ] Add a short note that restates the runtime versus developer tooling split.
  - [ ] Call out that `cefari-core` and `cefari-desktop` are runtime code.
  - [ ] Call out that `cefari-cli` is distributed separately as developer orchestration tooling.

## 2. Build `cefari-core`

- [ ] Scaffold the reusable runtime library crate.
  - [ ] Create `crates/cefari-core`.
  - [ ] Pin and add runtime dependencies: `serde`, `serde_json`, `directories`, `tracing`, `anyhow`, `thiserror`, `service-manager`, `cargo-packager-updater`, and `cargo-packager-resource-resolver`.
  - [ ] Define core error types with `thiserror`.
  - [ ] Define public result aliases and error conversion boundaries.
- [ ] Implement runtime path and config support.
  - [ ] Resolve config, data, cache, log, resource, and update artifact paths.
  - [ ] Add configuration schema structs.
  - [ ] Add configuration loading and saving.
  - [ ] Add JSON parsing and validation tests for configuration data.
- [ ] Implement runtime resource support.
  - [ ] Wrap `cargo-packager-resource-resolver`.
  - [ ] Define missing-resource error behavior.
  - [ ] Add host-independent tests where possible.
- [ ] Implement runtime logging support.
  - [ ] Define logging configuration consumed by `cefari-desktop`.
  - [ ] Provide helpers for log file paths and tracing setup inputs.
  - [ ] Document which logging setup remains desktop-owned.
- [ ] Implement update support.
  - [ ] Define update-check configuration and state types.
  - [ ] Implement update check helpers using `cargo-packager-updater`.
  - [ ] Implement update install helpers with clear failure states.
  - [ ] Add tests around update state and error mapping where possible.
- [ ] Implement service management support.
  - [ ] Define service operations for install, start, stop, restart, status, and uninstall.
  - [ ] Implement service helpers using `service-manager`.
  - [ ] Add tests for platform-independent service configuration behavior.
- [ ] Document the `cefari-core` API surface.
  - [ ] Add crate-level docs.
  - [ ] Mark which APIs are stable for `cefari-desktop`.
  - [ ] Keep internal helpers private unless a runtime caller needs them.

## 3. Build `cefari-desktop`

- [ ] Scaffold the shipped desktop app crate.
  - [ ] Create `crates/cefari-desktop`.
  - [ ] Add a path dependency on `cefari-core`.
  - [ ] Pin and add desktop dependencies: `tao`, `cef`, `raw-window-handle`, `single-instance`, `tracing-subscriber`, `tracing-appender`, `anyhow`, `muda`, `tray-icon`, and `open`.
- [ ] Implement desktop startup.
  - [ ] Initialize runtime logging through `cefari-core`.
  - [ ] Implement single-instance locking.
  - [ ] Define startup error reporting behavior before the UI is available.
- [ ] Implement the native shell.
  - [ ] Create the Tao event loop.
  - [ ] Create the main application window.
  - [ ] Initialize CEF in the desktop process.
  - [ ] Load packaged UI resources promptly at startup.
  - [ ] Add a fallback or diagnostic view for missing UI resources.
- [ ] Implement desktop integration.
  - [ ] Wire native menus through `muda`.
  - [ ] Wire tray or menu-bar icon behavior through `tray-icon`.
  - [ ] Add external link and file opening helpers through `open`.
- [ ] Integrate runtime operations.
  - [ ] Wire update check and install flow through `cefari-core`.
  - [ ] Wire daemon service install/start/stop/status behavior through `cefari-core`.
- [ ] Guard desktop-only boundaries.
  - [ ] Add smoke tests or compile-time checks for desktop-only dependency placement.
  - [ ] Confirm Tao and CEF dependencies are not introduced into `cefari-core` or `cefari-cli`.

## 4. Build `cefari-cli`

- [ ] Scaffold the developer-facing CLI crate.
  - [ ] Create `crates/cefari-cli`.
  - [ ] Set the binary name to `cefari`.
  - [ ] Pin and add CLI dependencies: `clap`, `anyhow`, `xshell`, `duct`, `camino`, `serde`, and `toml`.
  - [ ] Define the top-level CLI parser and command enum.
- [ ] Implement project creation.
  - [ ] Implement `cefari init`.
  - [ ] Define the generated Cefari project layout.
  - [ ] Add template files for frontend, daemon, desktop config, and package metadata.
  - [ ] Add fixture-based tests for generated project scaffolds.
- [ ] Implement development orchestration.
  - [ ] Implement `cefari dev`.
  - [ ] Run the frontend dev server.
  - [ ] Run the Deno daemon.
  - [ ] Run the Rust desktop app.
  - [ ] Handle process shutdown and error reporting.
- [ ] Implement build orchestration.
  - [ ] Implement `cefari build`.
  - [ ] Build frontend artifacts.
  - [ ] Build Deno daemon artifacts.
  - [ ] Build the Rust desktop app.
  - [ ] Implement CEF download and preparation as a CLI-owned step.
- [ ] Implement packaging and release commands.
  - [ ] Implement `cefari package` to invoke `cargo-packager`.
  - [ ] Implement `cefari codesign` to invoke `cargo-codesign`.
  - [ ] Implement `cefari notarize` for platform notarization flow.
  - [ ] Implement `cefari make-update` to generate update artifacts.
  - [ ] Add clean and dist task support if needed by build/package workflows.
- [ ] Implement diagnostics.
  - [ ] Implement `cefari doctor`.
  - [ ] Implement `cefari info`.
  - [ ] Provide clear errors when external tools such as `cargo-packager` or `cargo-codesign` are missing.
- [ ] Test CLI behavior.
  - [ ] Add integration tests for argument parsing.
  - [ ] Add integration tests for command dispatch.
  - [ ] Confirm `clap` and CLI-only orchestration dependencies are not introduced into runtime crates.

## 5. Create The Packaging And Release Pipeline

- [ ] Define package assembly.
  - [ ] Define package metadata consumed by `cargo-packager`.
  - [ ] Define how generated frontend artifacts are copied into packaged resources.
  - [ ] Define how generated Deno daemon artifacts are included in app packages.
  - [ ] Define how CEF binaries and resources are resolved during package creation.
- [ ] Add CI coverage.
  - [ ] Add CI steps for formatting, linting, testing, and workspace builds.
  - [ ] Add CI steps that install or provide `cargo-packager` and `cargo-codesign`.
  - [ ] Add platform-specific packaging jobs.
- [ ] Add release automation.
  - [ ] Add signing and notarization jobs for supported platforms.
  - [ ] Add update artifact generation and publishing jobs.
  - [ ] Verify release packages contain `cefari-desktop`, `cefari-core` runtime code, CEF resources, and generated app artifacts.
  - [ ] Verify `cefari-cli` is built, versioned, and distributed separately from desktop app packages.

## 6. Keep Dependencies Honest

- [ ] Pin implementation dependency versions.
  - [ ] Replace every README placeholder dependency version with a real version.
  - [ ] Use workspace dependency declarations where it improves consistency.
  - [ ] Decide whether shared versions should be centralized in workspace dependencies.
- [ ] Audit crate boundaries.
  - [ ] Confirm runtime crates do not depend on developer orchestration crates.
  - [ ] Confirm CLI-only crates do not pull in Tao or CEF.
  - [ ] Add dependency review notes for native and packaging crates.
  - [ ] Keep new dependencies out unless they directly serve the README architecture.

## 7. Maintain Documentation

- [ ] Update project documentation after scaffolding.
  - [ ] Update `README.md` to reflect actual commands and crate paths.
  - [ ] Document runtime versus CLI responsibility boundaries in contributor-facing docs.
- [ ] Document developer workflows.
  - [ ] Add CLI usage documentation for each `cefari` command.
  - [ ] Add development setup documentation for CEF preparation.
  - [ ] Add troubleshooting documentation for common `cefari doctor` failures.
- [ ] Document release workflows.
  - [ ] Add packaging documentation.
  - [ ] Add signing and notarization documentation.
  - [ ] Add update release documentation.

## 8. Verification Milestones

- [ ] Verify workspace health.
  - [ ] `cargo fmt --all` passes.
  - [ ] `cargo clippy --workspace --all-targets` passes.
  - [ ] `cargo test --workspace` passes.
- [ ] Verify CLI workflows.
  - [ ] `cargo run -p cefari-cli -- --help` shows all planned commands.
  - [ ] `cargo run -p cefari-cli -- init` creates a valid sample app.
  - [ ] `cargo run -p cefari-cli -- doctor` reports required tool availability.
- [ ] Verify desktop runtime behavior.
  - [ ] `cargo run -p cefari-desktop` starts a window and initializes runtime logging.
  - [ ] A development app can load UI resources through the desktop shell.
  - [ ] Service management operations are verified on each supported platform.
- [ ] Verify package and update behavior.
  - [ ] A packaged app contains the expected runtime, CEF, UI, and daemon artifacts.
  - [ ] An update artifact can be generated and consumed by the runtime update flow.

## 9. Open Decisions

- [ ] Decide supported operating systems and platform priority.
- [ ] Decide the frontend stack expected by `cefari init`.
- [ ] Decide the Deno daemon project shape and build output contract.
- [ ] Decide CEF version pinning and download source.
- [ ] Decide package identifiers, signing identities, and notarization requirements.
- [ ] Decide update server/artifact hosting expectations.
- [ ] Decide whether `cefari-cli` should support plugins or project hooks.
- [ ] Decide compatibility promises for generated project templates.
