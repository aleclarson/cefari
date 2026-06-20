# Documentation Verification

This page records the evidence used to keep the documentation grounded in the
current repository state.

Last checked: 2026-06-20.

## Source Evidence

Docs were checked against:

- `npm/src/cli.ts` for command names, arguments, and scaffold
  outputs
- `npm/src/config.ts` for `cefari.config.ts` loading,
  Deno execution, and runtime validation
- `npm/src/build.ts`, `package.ts`, `dev.ts`, and
  `release.rs` for guide behavior
- `templates/vite-react-basic/cefari.config.ts`, `deno.json`, frontend
  workspace manifest, and template workflows
- `.github/actions/cefari-release/action.yml` and
  `.github/actions/cefari-release/src/main.ts`
- `crates/cefari-core/src/ipc/`, `crates/cefari-core/src/ipc/capabilities/`,
  `crates/cefari-core/build.rs`, and `crates/cefari-core/bindings/ipc.ts`
- `crates/cefari-desktop/src/desktop_bridge.rs`, `desktop_ipc/`,
  `desktop_menu.rs`, `desktop_tray.rs`, `desktop_single_instance.rs`,
  `desktop_notifications.rs`, `desktop_dialogs.rs`, `event_loop.rs`,
  `window.rs`, and `window_state.rs`
- `npm/src/app/mod.ts`, namespace wrapper modules, and
  `npm/tests/app/cefari_app_test.ts`
- `npm/src/logs.ts`, `npm/src/logs-cli.ts`, `npm/src/sentry-logs.ts`, and
  `npm/tests/cli.test.ts` for SQLite log export and Sentry-shaped mapping

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
cargo test -p cefari-core
cargo test -p cefari-desktop
pnpm --dir npm check
pnpm --dir npm test
```

For the Sentry-shaped log export sprint on 2026-06-20, the final automated
checks were:

```bash
pnpm --dir npm exec tsc -p tsconfig.json --noEmit
pnpm --dir npm build && node --test npm/dist/tests/cli.test.js
pnpm --dir npm test
pnpm --dir npm check
deno test -A npm/tests/app/cefari_logs_test.ts
```

The no-network coverage creates app, daemon, worker, and Cefari runtime rows in
one SQLite database, exports them through one fake Sentry sink, verifies
Sentry-shaped levels and Cefari attributes, and checks that the export cursor
advances only after successful flush.

Human-run Sentry verification still requires a real project DSN:

```bash
export SENTRY_DSN="https://..."
export SENTRY_ENVIRONMENT="production"
export SENTRY_RELEASE="my-app@1.2.3"
cefari logs export sentry --once --level debug
```

Expected Sentry results:

- exported logs appear under the configured environment and release
- `log` level rows appear as Sentry `info`
- structured properties remain queryable as log attributes
- `cefari.scope`, `cefari.log_id`, and `cefari.pid` are present on each row

## Deep Link Verification

Automated checks cover deep-link config validation, generated package metadata,
runtime config output, typed IPC events, event filtering, primary runtime URL
classification, and secondary-process forwarding helpers.

Manual packaged-app checks are still required for OS protocol registration:

```bash
cefari build /path/to/project --release
cefari package /path/to/project --release
```

After installing or launching the packaged app, open a configured URL such as
`myapp://open/item`. Expected behavior:

- the app opens if it was not running
- the existing app window focuses if it was already running
- the frontend receives `cefari.on("deepLinkOpened", ...)`
- unconfigured custom schemes are ignored by Cefari

Platform notes:

- macOS: verify from Finder, Terminal `open 'myapp://open/item'`, or a browser
  prompt after installing the packaged app.
- Windows: verify from the Run dialog or `start myapp://open/item` after
  installing the packaged app.
- Linux: verify through the desktop environment or `xdg-open
  'myapp://open/item'` after installing the packaged app.

For the multi-window feature, the current focused verification set is:

```bash
cargo test -p cefari-core
cargo test -p cefari-desktop
pnpm --dir npm test
```

For the full notification dispatch sweep on 2026-06-17, the final automated
checks were:

```bash
cargo test -p cefari-core
cargo test -p cefari-desktop
pnpm --dir npm check
pnpm --dir npm test
```

The notification-specific automated coverage includes:

- generated Rust/TypeScript IPC binding sync
- TypeScript notification namespace command wrappers and response events
- desktop IPC dispatch for permission, capabilities, category registration,
  send, active listing, and delivered removal
- adapter validation for rich request fields, media path boundaries,
  category/action fields, response mapping, active listing, and removal
- CEF event injection script serialization
- packaged notification activation protocol metadata
- Windows-style activation URL decoding into `notification.response`

## Manual Notification Checks

Some notification behavior is intentionally manual because it depends on OS
services, signing credentials, or a live desktop session.

macOS:

- Build a packaged `.app` with a real `app.identifier`.
- Sign and notarize the bundle before validating real notification delivery.
- From a user-visible button, call `requestPermission()` and confirm the system
  prompt and resulting permission state.
- Send notifications with subtitle, image, thread id, category action, inline
  reply, and user info.
- Confirm default clicks emit `notification.response` and focus the main
  window.
- Confirm dismiss responses emit `notification.response` without focusing the
  main window.
- Confirm active listing and delivered removal affect Notification Center.

Windows:

- Package the app and inspect `cargo-packager.toml` for the generated
  `cefari-notification-...` deep-link protocol.
- Install the package so the AppUserModelID and protocol handler are registered.
- Send toast notifications with title/body, subtitle, image/icon, rounded icon,
  actions where supported, and user info.
- Confirm in-process click/action responses reach
  `cefari.notifications.onResponse`.
- Trigger a toast activation while the app is closed and confirm the activation
  URL decodes to the same `notification.response` shape.
- Confirm active listing and delivered removal work for delivered toasts.

Linux/XDG:

- Run inside a desktop session with a notification daemon that supports actions.
- Send title/body notifications plus image/icon and XDG category fields.
- Confirm daemon-specific behavior for action callbacks, close callbacks, and
  user info.
- Confirm active listing and removal are session-scoped.

Live CEF smoke can verify frontend event delivery without an OS prompt by
injecting a synthetic `notification.response` event through the CEF event helper
after the main frame has loaded.

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

To include the multi-window vertical slice, add `CEFARI_SMOKE_CREATE_WINDOW=1`:

```bash
CEFARI_LIVE_CEF_SMOKE=1 \
CEFARI_SMOKE_CREATE_WINDOW=1 \
CEFARI_SMOKE_EXIT_AFTER_MS=8000 \
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
- when `CEFARI_SMOKE_CREATE_WINDOW=1` is set, creates a secondary native window
  for `/smoke-secondary`
- captures process stdout and stderr under `.tmp/cef-live-smoke/`
- exits the desktop process through `CEFARI_SMOKE_EXIT_AFTER_MS`

The fixture reloads once and then sets the native window title to
`Cefari Smoke PASS`. A local human run should verify that title appears before
the process exits. A nonzero process exit status, missing CEF resources, missing
GUI availability, or a watchdog timeout fails the smoke.

CEF download behavior requires a local GUI check because it uses the OS save
dialog. To verify it manually, run a Cefari app with a link to an HTTP or HTTPS
download response, click it, choose a destination in the save dialog, and
confirm that `download.started`, `download.progress`, and `download.completed`
events reach the frontend. Repeat with the save dialog canceled and with a
`file:` or `blob:` download URL; those cases should not write a file and should
surface a canceled or denied outcome.

Manual platform verification for parent/modal windows should cover:

- Windows owner-window behavior and parent disabling while a modal child is live
- macOS parent-window ordering; document-modal sheets are not part of this
  supported slice
- Linux transient-window behavior on the active X11 or Wayland backend
- Wayland position persistence, where absolute positions may be unavailable

The docs intentionally avoid listing dependency versions, exhaustive internal
module details, or CI behavior that is not present in the current repository
files.
