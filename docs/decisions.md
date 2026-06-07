# Project Decisions

These decisions capture the current implementation state. They should be revised when the corresponding implementation grows beyond the current scaffold.

## Platform Priority

Cefari currently verifies the workspace on macOS first. The desktop startup path has been run on macOS and creates:

- a runtime log file under `~/Library/Application Support/dev.Cefari.Cefari/logs/`
- a single-instance lock file under `~/Library/Caches/dev.Cefari.Cefari/`

Before the UI is available, desktop startup failures are reported to stderr with a stable `Cefari failed to start before the UI was available` prefix and, when tracing has already initialized, to the runtime log.

The desktop process now resolves `frontend/index.html` from platform-appropriate package resource directories or the runtime resource directory before the main window is created. If the UI entry is missing, startup writes a cache-backed diagnostic HTML file and marks the window title as `Cefari - Missing UI Resources`. CEF initialization is still a separate native shell task.

External URL and file open requests are routed through the desktop-only `open` dependency. URL helpers currently allow `http`, `https`, and `mailto` schemes; file helpers validate local path existence before launching the platform opener.

Native menus are built with `muda` during desktop startup. On macOS the menu is installed as the application menu, menu events are forwarded through Tao user events, and implemented menu actions currently quit the app or open the runtime log directory. Update-check, UI reload, and service-status menu entries are wired to the event path and currently log activation until the corresponding UI/runtime flows are ready to run from the menu.

Tray/menu-bar icon behavior is built with `tray-icon`. The tray icon is created after Tao emits `StartCause::Init`, has a small generated template icon and context menu, routes tray events into the Tao user-event loop, restores/focuses the main window on primary click, and shares the menu command IDs used by the native app menu.

Desktop runtime operations are adapted through `cefari-core`: update checks/install calls use `UpdateCheckConfig`, `check_for_update`, and `install_update`, while daemon service operations build a `CefariServiceSpec` and call the core service-manager wrappers. These adapters are prepared at startup but side-effecting operations are reserved for later UI or menu triggers.

Windows and Linux remain target platforms, but packaging, service operations, and desktop shell behavior still need platform-specific verification before they can be treated as supported.

## Frontend Template

`cefari init` currently generates a minimal static frontend:

- `frontend/index.html`
- configured dist path: `frontend/dist`

No JavaScript framework is selected yet. A richer template should be introduced only when `cefari dev` and `cefari build` define how frontend commands are run.

`cefari dev` currently serves this static frontend with a built-in local HTTP server instead of assuming a Node-based frontend toolchain. A future template that selects a JavaScript framework should replace or extend this with project-configured frontend commands.

## Deno Daemon Shape

`cefari init` currently generates:

- `daemon/main.ts`
- configured daemon entry: `daemon/main.ts`

`cefari build` currently copies the daemon entry to `build/daemon/main.ts`.

`cefari dev` runs the daemon entry with `deno run --watch --allow-read --allow-net`.

Final packaged releases should not rely on a developer Deno installation or treat TypeScript source as the service executable. The final packaged daemon output contract is a platform executable produced from the configured Deno entry and copied into `build/daemon/` with a stable executable name. Until that compile step is implemented, the current TypeScript copy remains a pre-release build artifact only.

`cefari-cli` should not support plugins or arbitrary project hooks before the first public release. The current command surface stays explicit and deterministic; extension points should be reconsidered only after the core init, dev, build, package, signing, notarization, and update workflows are stable.

## CEF Version

The workspace currently pins `cef = "148.4.0"`.

The dependency is optional in `cefari-desktop` until CEF initialization is implemented and verified in the desktop process.

CEF preparation is owned by `cefari-cli`. The initial download source should be selected from the CEF distribution expected by the pinned Rust `cef` crate version. Automated download and cache behavior is still tracked separately in `todo.md`.

`cefari build` creates `build/cef/manifest.json` and `build/cef/resources/` so package assembly can resolve a deterministic CEF resource path. The manifest records `source = "pending-download"` until the large binary fetch/cache step is implemented.

## Package Identifiers And Signing

Generated projects use the app identifier from `cefari.toml` as the package identifier. `cefari init` currently derives that identifier from the display name, using the `dev.cefari.<slug>` shape.

Signing identities are not hard-coded into generated projects. They are supplied by developer environment, CI configuration, or an explicit `sign.toml` path passed to `cefari codesign` and `cefari notarize`.

## Update Artifacts

Update artifact hosting is external to the app package. Runtime update checks consume configured endpoints and public keys; release automation is responsible for publishing compatible update metadata and signed artifacts.

`cefari make-update` signs a release archive through `cargo-codesign` and writes metadata that matches the `cargo-packager-updater` response shape documented by that crate.

## Generated Template Compatibility

Generated project templates are pre-release and may change before the first public distribution of `cefari-cli`.

Until compatibility is formalized, tests only guarantee that the current CLI can parse the current generated `cefari.toml` schema.
