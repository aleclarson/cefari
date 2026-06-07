# Documentation Verification

This page records the evidence used to keep the documentation grounded in the current repository state.

Last checked: 2026-06-07.

## Source Evidence

Docs were checked against:

- `crates/cefari-cli/src/lib.rs` for command names, arguments, and scaffold outputs
- `crates/cefari-cli/src/project.rs` for `cefari.toml` parsing and defaults
- `crates/cefari-cli/src/build.rs`, `package.rs`, `dev.rs`, `clean.rs`, and `release.rs` for guide behavior
- `templates/vite-react-basic/cefari.toml`, `deno.json`, frontend and daemon workspace manifests, and template workflows
- `.github/actions/cefari-release/action.yml` and `.github/actions/cefari-release/release.sh`
- `crates/cefari-core/src/ipc.rs` and `crates/cefari-core/bindings/ipc.ts`
- `crates/cefari-desktop/src/desktop_bridge.rs`, `desktop_ipc.rs`, `desktop_menu.rs`, `desktop_tray.rs`, and `desktop_notifications.rs`

## Command Evidence

These checks were run during the documentation cleanup:

```bash
cargo run -p cefari-cli -- --help
cargo run -p cefari-cli -- init /tmp/cefari-docs-smoke --name "Docs Smoke"
cargo run -p cefari-cli -- info
cargo run -p cefari-cli -- doctor
deno task --cwd templates/vite-react-basic/frontend check
actionlint docs/examples/cefari-release-workflow.yml templates/vite-react-basic/.github/workflows/release.yml templates/vite-react-basic/.github/workflows/prerelease.yml
```

The docs intentionally avoid listing dependency versions, exhaustive internal module details, or CI behavior that is not present in the current repository files.
