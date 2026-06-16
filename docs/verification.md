# Documentation Verification

This page records the evidence used to keep the documentation grounded in the
current repository state.

Last checked: 2026-06-13.

## Source Evidence

Docs were checked against:

- `npm/src/cli.ts` for command names, arguments, and scaffold
  outputs
- `npm/src/config.ts` for `cefari.config.ts` loading,
  Deno execution, and runtime validation
- `npm/src/build.ts`, `package.ts`, `dev.ts`, and
  `release.rs` for guide behavior
- `templates/vite-react-basic/cefari.config.ts`, `deno.json`, frontend and daemon
  workspace manifests, and template workflows
- `.github/actions/cefari-release/action.yml` and
  `.github/actions/cefari-release/src/main.ts`
- `crates/cefari-core/src/ipc.rs` and `crates/cefari-core/bindings/ipc.ts`
- `crates/cefari-desktop/src/desktop_bridge.rs`, `desktop_ipc.rs`,
  `desktop_menu.rs`, `desktop_tray.rs`, and `desktop_notifications.rs`
- `npm/src/app/mod.ts`, namespace wrapper modules, and
  `npm/tests/app/cefari_app_test.ts`

## Command Evidence

These checks were run during the documentation cleanup:

```bash
pnpm add -g ./npm
cefari --help
cp -R templates/vite-react-basic /tmp/cefari-docs-smoke
cefari package
cefari --help
deno run -A npm/dist/bin/cefari.js --help
pnpm --dir npm test
cargo test -p cefari-desktop desktop_notifications
deno task --cwd templates/vite-react-basic/frontend check
actionlint docs/examples/cefari-release-workflow.yml templates/vite-react-basic/.github/workflows/release.yml templates/vite-react-basic/.github/workflows/prerelease.yml
actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/release.yml templates/vite-react-basic/.github/workflows/prerelease.yml docs/examples/cefari-release-workflow.yml
deno task --cwd .github/actions/cefari-release check
cp -R templates/vite-react-basic .ci/release-action-sample
GITHUB_OUTPUT=/tmp/cefari-release-output.txt GITHUB_ACTION_PATH="$PWD/.github/actions/cefari-release" CEFARI_PROJECT_PATH=templates/vite-react-basic CEFARI_RELEASE_MODE=prerelease CEFARI_TARGETS=linux-x86_64 CEFARI_COMMAND=cefari CEFARI_INSTALL_CLI=false CEFARI_RELEASE_VERSION=0.0.0-ci CEFARI_RELEASE_TAG=release-action-ci-dry-run CEFARI_CREATE_GITHUB_RELEASE=false CEFARI_UPLOAD_ARTIFACTS=false CEFARI_DRY_RUN=true deno run -A --config .github/actions/cefari-release/deno.json .github/actions/cefari-release/src/main.ts
cargo test -p cefari-core ipc::tests::generated_typescript_bindings_are_current
cargo test -p cefari-desktop
pnpm --dir npm check
pnpm --dir npm test
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
