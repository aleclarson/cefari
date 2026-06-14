# Documentation Verification

This page records the evidence used to keep the documentation grounded in the
current repository state.

Last checked: 2026-06-13.

## Source Evidence

Docs were checked against:

- `crates/cefari-cli/src/lib.rs` for command names, arguments, and scaffold
  outputs
- `crates/cefari-cli/src/project.rs` for `cefari.toml` parsing and defaults
- `crates/cefari-cli/src/build.rs`, `package.rs`, `dev.rs`, `clean.rs`, and
  `release.rs` for guide behavior
- `templates/vite-react-basic/cefari.toml`, `deno.json`, frontend and daemon
  workspace manifests, and template workflows
- `.github/actions/cefari-release/action.yml` and
  `.github/actions/cefari-release/release.sh`
- `crates/cefari-core/src/ipc.rs` and `crates/cefari-core/bindings/ipc.ts`
- `crates/cefari-desktop/src/desktop_bridge.rs`, `desktop_ipc.rs`,
  `desktop_menu.rs`, `desktop_tray.rs`, and `desktop_notifications.rs`
- `packages/cefari-app/src/mod.ts`, namespace wrapper modules, and
  `packages/cefari-app/tests/cefari_app_test.ts`

## Command Evidence

These checks were run during the documentation cleanup:

```bash
cargo install --path crates/cefari-cli --locked --root /tmp/cefari-cli-docs-install
PATH="/tmp/cefari-cli-docs-install/bin:$PATH" cefari --help
PATH="/tmp/cefari-cli-docs-install/bin:$PATH" cefari init /tmp/cefari-docs-smoke --name "Docs Smoke"
PATH="/tmp/cefari-cli-docs-install/bin:$PATH" cefari info
PATH="/tmp/cefari-cli-docs-install/bin:$PATH" cefari doctor
cargo run -q -p cefari-cli -- --help
cargo test -p cefari-cli
cargo test -p cefari-desktop desktop_notifications
deno task --cwd templates/vite-react-basic/frontend check
actionlint docs/examples/cefari-release-workflow.yml templates/vite-react-basic/.github/workflows/release.yml templates/vite-react-basic/.github/workflows/prerelease.yml
actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/release.yml templates/vite-react-basic/.github/workflows/prerelease.yml docs/examples/cefari-release-workflow.yml
shellcheck .github/actions/cefari-release/release.sh
bash -n .github/actions/cefari-release/release.sh
cargo run -p cefari-cli -- init .ci/release-action-sample --name "Release Action CI"
GITHUB_OUTPUT=/tmp/cefari-release-output.txt GITHUB_ACTION_PATH="$PWD/.github/actions/cefari-release" CEFARI_PROJECT_PATH=templates/vite-react-basic CEFARI_RELEASE_MODE=prerelease CEFARI_TARGETS=linux-x86_64 CEFARI_COMMAND="$PWD/target/debug/cefari" CEFARI_INSTALL_CLI=false CEFARI_RELEASE_VERSION=0.0.0-ci CEFARI_RELEASE_TAG=release-action-ci-dry-run CEFARI_CREATE_GITHUB_RELEASE=false CEFARI_UPLOAD_ARTIFACTS=false CEFARI_DRY_RUN=true .github/actions/cefari-release/release.sh
cargo test -p cefari-core ipc::tests::generated_typescript_bindings_are_current
cargo test -p cefari-desktop
deno task --cwd packages/cefari-app check
deno task --cwd packages/cefari-app test
```

## Release Action CI

The `Release Action Validation` CI job lint-checks the shared release action,
template workflows, and example workflow. It also runs the action script in
dry-run mode, then invokes the composite action against a generated app fixture
with GitHub release creation disabled and GitHub Actions artifact upload
enabled.

The real action invocation verifies:

- release action outputs are populated
- `dist/release-artifacts.txt` exists and lists package output
- package metadata uses the requested release version
- native package payload inspection passes for the generated fixture

This keeps PR validation credential-free. Production signing, notarization, and
GitHub release publication still require repository secrets and tag or manual
release workflows.

## Live CEF Smoke

Run the live CEF smoke only on a machine with a GUI session and extracted CEF
resources:

```bash
CEFARI_LIVE_CEF_SMOKE=1 \
CEFARI_CEF_RESOURCES_DIR=/path/to/build/cef/resources \
scripts/cef-live-smoke.sh
```

When `CEFARI_LIVE_CEF_SMOKE` is not set, the command exits successfully with a
skip message so CI can include it without requiring local CEF binaries. When it
does run, the script:

- builds `cefari-desktop`
- creates a minimal fixture frontend under `.tmp/cef-live-smoke/resources`
- loads that fixture through `cefari://app/index.html` by setting
  `CEFARI_RESOURCE_DIR`
- verifies in page JavaScript that `window.cefari` exists
- invokes harmless native IPC commands: `updateState`, `reloadUi`, and
  `windowSetTitle`
- captures process stdout and stderr under `.tmp/cef-live-smoke/`
- exits the desktop process through `CEFARI_SMOKE_EXIT_AFTER_MS`

The fixture reloads once and then sets the native window title to
`Cefari Smoke PASS`. A local human run should verify that title appears before
the process exits. A nonzero process exit status, missing CEF resources, missing
GUI availability, or a watchdog timeout fails the smoke.

The docs intentionally avoid listing dependency versions, exhaustive internal
module details, or CI behavior that is not present in the current repository
files.
