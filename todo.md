# Cefari Implementation Todo

This task list is derived from [README.md](README.md). It organizes the path from the architecture plan to a working Rust workspace with runtime crates, a developer CLI, packaging flow, and release/update support.

## 1. Workspace Foundation

- [ ] Create the Rust workspace root `Cargo.toml`.
- [ ] Add workspace members for `crates/cefari-core`, `crates/cefari-desktop`, and `crates/cefari-cli`.
- [ ] Set shared workspace metadata: edition, license, repository, authors, rust-version, and package defaults.
- [ ] Decide and document the minimum supported Rust version.
- [ ] Add baseline `.gitignore` entries for Rust, build artifacts, generated packages, CEF downloads, logs, and local environment files.
- [ ] Add a root formatting and linting policy using `rustfmt` and `clippy`.
- [ ] Add a root `justfile`, `Makefile`, or script directory only if it matches the intended developer workflow.
- [ ] Add a short architecture note that restates the runtime/developer tooling boundary for future contributors.

## 2. `cefari-core` Runtime Library

- [ ] Scaffold the `cefari-core` library crate.
- [ ] Pin and add runtime dependencies: `serde`, `serde_json`, `directories`, `tracing`, `anyhow`, `thiserror`, `service-manager`, `cargo-packager-updater`, and `cargo-packager-resource-resolver`.
- [ ] Define core error types with `thiserror`.
- [ ] Define public result aliases and error conversion boundaries.
- [ ] Implement application path resolution for config, data, cache, logs, resources, and update artifacts.
- [ ] Add configuration loading and saving with explicit schema structs.
- [ ] Add JSON parsing and validation tests for configuration data.
- [ ] Implement resource resolution wrappers around `cargo-packager-resource-resolver`.
- [ ] Add runtime logging helpers that can be consumed by `cefari-desktop`.
- [ ] Define update-check configuration and state types.
- [ ] Implement update check helpers using `cargo-packager-updater`.
- [ ] Implement update install helpers with clear failure states.
- [ ] Define service management abstractions for install, start, stop, restart, status, and uninstall.
- [ ] Implement service helpers using `service-manager`.
- [ ] Add unit tests for path, config, resource, update, and service helper behavior where host-independent tests are possible.
- [ ] Document which APIs are stable for `cefari-desktop` and which remain internal.

## 3. `cefari-desktop` Shipped App

- [ ] Scaffold the `cefari-desktop` binary crate.
- [ ] Add a path dependency on `cefari-core`.
- [ ] Pin and add desktop dependencies: `tao`, `cef`, `raw-window-handle`, `single-instance`, `tracing-subscriber`, `tracing-appender`, `anyhow`, `muda`, `tray-icon`, and `open`.
- [ ] Implement runtime logging initialization through `cefari-core`.
- [ ] Implement single-instance locking.
- [ ] Create the Tao event loop and main application window.
- [ ] Initialize CEF in the desktop process.
- [ ] Load packaged UI resources promptly at startup.
- [ ] Add a fallback or diagnostic view for missing UI resources.
- [ ] Wire native menus through `muda`.
- [ ] Wire tray or menu-bar icon behavior through `tray-icon`.
- [ ] Add external link and file opening helpers through `open`.
- [ ] Integrate Rust-side update check and install flow through `cefari-core`.
- [ ] Integrate daemon service install/start/stop/status behavior through `cefari-core`.
- [ ] Define startup error reporting behavior before the UI is available.
- [ ] Add smoke tests or compile-time checks for desktop-only dependency placement.
- [ ] Confirm Tao and CEF dependencies are not introduced into `cefari-core` or `cefari-cli`.

## 4. `cefari-cli` Developer Tool

- [ ] Scaffold the `cefari-cli` binary crate with binary name `cefari`.
- [ ] Pin and add CLI dependencies: `clap`, `anyhow`, `xshell`, `duct`, `camino`, `serde`, and `toml`.
- [ ] Define the top-level CLI parser and command enum.
- [ ] Implement `cefari init` for project scaffolding.
- [ ] Define the generated Cefari project layout.
- [ ] Add template files for frontend, daemon, desktop config, and package metadata.
- [ ] Implement `cefari dev` to run the local development environment.
- [ ] Add process orchestration for frontend dev server, daemon, and desktop app.
- [ ] Implement `cefari build` for frontend, Deno daemon, and Rust desktop builds.
- [ ] Implement `cefari package` to invoke `cargo-packager`.
- [ ] Implement `cefari codesign` to invoke `cargo-codesign`.
- [ ] Implement `cefari notarize` for platform notarization flow.
- [ ] Implement `cefari make-update` to generate update artifacts.
- [ ] Implement `cefari doctor` for environment diagnostics.
- [ ] Implement `cefari info` for project and toolchain information.
- [ ] Implement CEF download and preparation as a CLI-owned step.
- [ ] Add clean and dist task support if needed by build/package workflows.
- [ ] Provide clear errors when external tools such as `cargo-packager` or `cargo-codesign` are missing.
- [ ] Add CLI integration tests for argument parsing and command dispatch.
- [ ] Add fixture-based tests for generated project scaffolds.
- [ ] Confirm `clap` and CLI-only orchestration dependencies are not introduced into runtime crates.

## 5. Build, Packaging, And Release

- [ ] Define package metadata consumed by `cargo-packager`.
- [ ] Define how generated frontend artifacts are copied into packaged resources.
- [ ] Define how generated Deno daemon artifacts are included in app packages.
- [ ] Define how CEF binaries and resources are resolved during package creation.
- [ ] Add CI steps for formatting, linting, testing, and workspace builds.
- [ ] Add CI steps that install or provide `cargo-packager` and `cargo-codesign`.
- [ ] Add platform-specific packaging jobs.
- [ ] Add signing and notarization jobs for supported platforms.
- [ ] Add update artifact generation and publishing jobs.
- [ ] Verify release packages contain `cefari-desktop`, `cefari-core` runtime code, CEF resources, and generated app artifacts.
- [ ] Verify `cefari-cli` is built, versioned, and distributed separately from desktop app packages.

## 6. Dependency Hygiene

- [ ] Replace every README placeholder dependency version with pinned implementation versions.
- [ ] Use workspace dependency declarations where it improves consistency.
- [ ] Audit that runtime crates do not depend on developer orchestration crates.
- [ ] Audit that CLI-only crates do not pull in Tao or CEF.
- [ ] Decide whether shared versions should be centralized in workspace dependencies.
- [ ] Add dependency review notes for native and packaging crates.
- [ ] Keep new dependencies out unless they directly serve the README architecture.

## 7. Documentation

- [ ] Update `README.md` after scaffolding to reflect actual commands and crate paths.
- [ ] Add crate-level docs for `cefari-core`.
- [ ] Add CLI usage documentation for each `cefari` command.
- [ ] Add development setup documentation for CEF preparation.
- [ ] Add packaging, signing, notarization, and update release documentation.
- [ ] Add troubleshooting documentation for common `cefari doctor` failures.
- [ ] Document runtime versus CLI responsibility boundaries in contributor-facing docs.

## 8. Verification Milestones

- [ ] `cargo fmt --all` passes.
- [ ] `cargo clippy --workspace --all-targets` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo run -p cefari-cli -- --help` shows all planned commands.
- [ ] `cargo run -p cefari-cli -- init` creates a valid sample app.
- [ ] `cargo run -p cefari-cli -- doctor` reports required tool availability.
- [ ] `cargo run -p cefari-desktop` starts a window and initializes runtime logging.
- [ ] A development app can load UI resources through the desktop shell.
- [ ] A packaged app contains the expected runtime, CEF, UI, and daemon artifacts.
- [ ] An update artifact can be generated and consumed by the runtime update flow.
- [ ] Service management operations are verified on each supported platform.

## 9. Open Decisions

- [ ] Decide supported operating systems and platform priority.
- [ ] Decide the frontend stack expected by `cefari init`.
- [ ] Decide the Deno daemon project shape and build output contract.
- [ ] Decide CEF version pinning and download source.
- [ ] Decide package identifiers, signing identities, and notarization requirements.
- [ ] Decide update server/artifact hosting expectations.
- [ ] Decide whether `cefari-cli` should support plugins or project hooks.
- [ ] Decide compatibility promises for generated project templates.
