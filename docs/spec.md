# Cefari Capability Spec

Cefari builds Vite applications into native desktop apps with a Deno daemon,
native desktop runtime, package assembly, updater support, and typed frontend
APIs.

## App Project Model

> This section explains how a Cefari app describes itself.

- Cefari apps are described by a `cefari.config.ts` file.
- The config can define the app's stable identity.
  - It sets the machine name used for generated executables.
  - It sets the human-readable app name used by runtime chrome and tooling.
  - It sets the reverse-DNS app identifier used by native integrations.
  - It can provide an app icon for package metadata.
- The config can define Vite behavior.
  - It sets the Vite project root.
  - It selects a Vite config file or disables config-file discovery.
  - It sets the fixed development server port.
- The config can define a Deno daemon entrypoint.
  - Dev mode runs the daemon from source.
  - Build mode compiles the daemon into an executable.
- The config can define package metadata.
  - It sets the packaged product name.
  - It sets the app version used by package metadata and update checks.
- The config can opt into native capabilities.
  - Tray and menu-bar integration is available when configured.
  - Deep-link URL schemes are available when configured.
  - Native capabilities are disabled unless the app opts in.
- Config paths are resolved relative to the app project.
  - Path fields must remain inside the project.
  - Generated output is written under the project root.

## Project Creation And Templates

> This section covers the starter app and example release workflows.

- Cefari includes a Vite React starter template.
  - The template has separate frontend and daemon workspaces.
  - The frontend is a Vite app.
  - The daemon is a Deno program.
  - The template imports typed Cefari frontend APIs.
- The template includes release workflow examples.
  - It includes a production release workflow.
  - It includes a prerelease workflow with dry-run support.
  - The workflows show platform matrix setup for macOS, Linux, and Windows.

## Local Development

> This section describes what Cefari runs while you build locally.

- Cefari can run an app in local development mode.
  - It starts the Vite dev server.
  - It starts the Deno daemon in watch mode.
  - It starts the native desktop runtime.
  - It stops the remaining child processes when one process exits or fails.
- Dev mode uses the app's configured Vite port by default.
  - The CLI can override the Vite port for a run.
  - The CLI can expose a Chrome DevTools Protocol port for the embedded CEF
    browser.
- Cefari injects a small CSS contract for custom titlebars.
  - Apps can mark a region as draggable.
  - Apps can mark interactive descendants as non-draggable.
  - The contract is opt-in and does not make app chrome draggable by default.

## Build

> This section lists the artifacts Cefari can produce for an app.

- Cefari can build all app runtime pieces.
  - It builds the Vite frontend into `build/frontend/`.
  - It writes runtime config into `build/config/cefari.json`.
  - It copies the daemon source entry into `build/daemon/main.ts`.
  - It compiles the daemon into a project-named executable.
  - It prepares a desktop runtime executable.
  - It prepares CEF resources and manifest data.
- Cefari can build with a release profile.
  - Release builds use Cargo's release profile when the desktop runtime is built
    from source.
- Cefari can use different desktop runtime sources.
  - Installed CLI distributions can bundle a matching desktop runtime.
  - Source checkouts can build the runtime with Cargo.
  - An environment variable can point Cefari at a specific prebuilt runtime.
- Cefari prepares CEF runtime payloads for packaging.
  - It gathers resource files.
  - It records archive metadata.
  - It validates expected CEF files before package assembly.

## Package Assembly

> This section explains how Cefari prepares native packages.

- Cefari can prepare native package assembly metadata.
  - It writes package metadata under `dist/package/`.
  - It writes a `cargo-packager` configuration.
  - It writes a package manifest.
- Cefari can invoke `cargo-packager` when it is available.
  - Native package output is written under `dist/package/output/`.
  - If `cargo-packager` is unavailable, Cefari leaves package metadata in place.
- Cefari can package release-profile build output.
- Cefari can override the package version for release packaging.
- Cefari validates package inputs before assembly.
  - It expects build artifacts to already exist.
  - It checks the desktop executable.
  - It checks runtime config.
  - It checks CEF resource metadata.
  - It checks for locale files.
  - It includes configured tray icons.
  - It registers configured deep-link URL schemes.

## Release Tooling

> This section covers signing, notarization, updates, and release automation.

- Cefari can sign packaged artifacts.
  - It supports macOS, Windows, and Linux signing targets.
  - It can read signing configuration from a `sign.toml` file.
  - It accepts macOS app bundles and disk images.
- Cefari can notarize macOS artifacts.
  - It notarizes signed macOS app bundles and disk images.
  - It uses signing configuration when notarization credentials are needed.
- Cefari can generate update metadata.
  - It signs a release archive.
  - It writes the archive signature.
  - It writes an `update.json` manifest.
  - It records the advertised version, target platform, package format,
    signature, and archive URL.
- Cefari can run a release packaging pipeline.
  - It can build and package a project.
  - It can sign artifacts.
  - It can notarize macOS artifacts.
  - It can generate updater metadata.
  - It can create or update a GitHub release.
  - It can run in dry-run mode and print the planned release steps.
- Cefari includes a composite GitHub Action for releases.
  - The action delegates build, package, signing, notarization, and update
    generation to the CLI.
  - The action can install a specific Cefari CLI version with pnpm.
  - The action can upload release artifacts.
  - The action can create or update GitHub releases.
  - The action can skip secret-dependent steps when required secrets are absent.

## Native Desktop Runtime

> This section describes the Rust runtime that ships with an app.

- Cefari ships a Rust desktop runtime.
  - It owns CEF startup.
  - It owns windowing.
  - It owns native menus.
  - It owns tray and menu-bar integration.
  - It owns notification setup.
  - It owns native action dispatch.
  - It owns runtime logging setup.
- Cefari embeds the app frontend in a native desktop shell.
  - The frontend talks to the runtime through `window.cefari`.
  - The runtime exposes a typed IPC contract.
  - The TypeScript package re-exports the generated IPC types.
- Cefari can receive configured OS deep links.
  - Configured URL schemes are registered in packaged apps.
  - Opened deep links are delivered to frontend code as events.
  - A second process can forward deep links to the already-running app.
- Cefari separates reusable runtime helpers from desktop concerns.
  - Shared config, paths, resources, logging inputs, services, updates, and IPC
    types live in the core crate.
  - Windowing, CEF startup, menus, tray behavior, notifications, and dispatch
    live in the desktop crate.
  - Developer tooling lives in the npm package.

## Frontend TypeScript APIs

> This section lists the typed APIs available to frontend code.

- Cefari exposes frontend APIs through `cefari/app`.
  - Apps can import a single `cefari` object.
  - Apps can import individual namespaces.
  - Apps can check whether the native bridge is available.
  - Apps can call low-level IPC commands directly when needed.
  - Apps can use throwing APIs or result-style APIs.
- Cefari exposes typed event subscriptions.
  - Apps can subscribe to named events.
  - Apps can subscribe to all native events.
  - Event subscriptions return unsubscribe functions.
  - Apps can subscribe to deep-link open events.
- Cefari exposes typed errors.
  - Unsupported native calls report a typed unsupported error.
  - Runtime IPC errors are wrapped as `CefariError` values.

## App And Window APIs

> This section covers app lifecycle and native window controls.

- Apps can ask the native runtime to quit.
- Apps can control the current native window through `cefari.window`.
  - Apps can read the current window state.
  - Apps can show the current window.
  - Apps can focus the current window.
  - Apps can close the current window.
  - Apps can set the current window title.
- Apps can create and manage secondary native windows through `cefari.windows`.
  - Secondary windows use Cefari string IDs.
  - The startup window is always `main`.
  - Apps can list live windows.
  - Apps can get a window by ID.
  - Apps can show, focus, close, and retitle a window by ID.
  - Apps can assign a parent window when creating a secondary window.
  - Modal windows require a valid parent window.
  - Closing a parent closes its child windows.
- Cefari persists native window geometry.
  - The `main` window geometry persists by default.
  - Secondary window geometry persists only when the app supplies `persistKey`.
  - Cefari persists size, position where supported, maximized state, and
    fullscreen state.
  - Cefari does not persist secondary window existence, routes, or parent
    relationships.
  - Invalid persisted geometry is ignored without blocking app startup.
- Secondary windows load trusted app frontend content.
  - Development windows resolve routes against the configured Vite dev URL.
  - Packaged windows load `cefari://app/index.html` and carry the route in URL
    metadata.
  - Arbitrary external URLs are not trusted app windows.
- Parent and modal behavior is best-effort across platforms.
  - Windows uses owner windows for dialog-like secondary windows.
  - macOS uses native parent-window ordering.
  - Linux uses transient windows where the backend supports it.
  - Cefari always tracks parent and modal state in `WindowState`.
- Apps can subscribe to window events.
  - Window events include the Cefari window ID.
  - Created, shown, focused, blurred, close-requested, closed, moved, resized,
    and title-changed events are available.
  - `cefari.windows` event helpers can filter by window ID.

## Shell APIs

> This section covers OS shell actions exposed to frontend code.

- Apps can open the runtime log location.
- Apps can ask the OS to open external URLs.
  - URLs are validated by Rust before opening.
  - String URLs and `URL` objects are accepted by the TypeScript wrapper.
- Apps can receive configured custom URL schemes from the OS.
  - Deep links are delivered as typed frontend events.
  - The event payload includes the opened URL string.
  - Unconfigured custom schemes are ignored by the runtime.
- Cefari reserves a UI reload command.
  - The frontend wrapper exists.
  - The current desktop dispatcher reports it as unsupported.

## Native Dialogs

> This section covers native file and folder selection dialogs.

- Apps can open native dialogs from the frontend.
  - They can choose one file.
  - They can choose multiple files.
  - They can choose one folder.
  - They can choose multiple folders.
  - They can choose a save path.
- Native dialogs support common dialog options.
  - Apps can set a dialog title.
  - Apps can set file extension filters.
  - Apps can set a default native directory.
  - Apps can set a default app-data directory.
  - Apps can set a default file name.
  - Apps can request main-window modality.
  - Apps can request directory creation when the platform supports it.
- Dialog cancellation is a normal result.
  - Canceling a dialog does not throw a frontend error.
  - Invalid dialog requests still report typed Cefari errors.
- Save dialogs select a path.
  - They do not write files.
  - They do not overwrite files.
- Native dialog paths are separate from app-data filesystem paths.
  - Selected native paths do not expand `cefari.fs` access.
  - App-data default directories use app-data path validation.
  - File filters are user-interface hints, not security boundaries.
- Native dialog behavior can vary by platform.
  - Cefari supports macOS, Linux, and Windows native dialogs.
  - Some option details depend on the operating system dialog backend.

## Downloads

> This section describes browser-initiated download behavior.

- Cefari handles CEF downloads in the native runtime.
  - Downloads use the OS save dialog before writing files.
  - HTTP and HTTPS downloads are supported.
  - Unsupported schemes are denied by the runtime.
- Apps can observe download lifecycle events.
  - A download-started event is available.
  - A download-progress event is available.
  - Download-completed, download-canceled, and download-failed events are
    available.
- Apps can control downloads through the TypeScript API.
  - Active downloads can be canceled by ID.
  - Completed downloads can be revealed through the OS shell.

## Updates

> This section describes update checks, apply flows, and update events.

- Apps can read updater state.
  - The updater can report that updates are not configured.
  - The updater can report that the app is current.
  - The updater can report that it is checking.
  - The updater can report that an update is available.
  - The updater can report that an update is applying.
  - The updater can report that an update is ready to restart.
  - The updater can report an error.
- Apps can trigger an update check.
- Apps can apply a checked update.
  - The runtime uses the update cached from the latest successful check.
  - The frontend does not pass update URLs or signatures into apply calls.
- Apps can restart after an update is ready.
- Apps can apply an update and restart as a single frontend action.
- Apps can subscribe to updater state changes.

## Daemon Service

> This section covers the Deno daemon and its frontend status API.

- Cefari can run a Deno daemon beside the frontend.
- Apps can read daemon service status from the frontend.
- Apps can subscribe to daemon service status changes.
- The current frontend wrapper does not expose daemon start, stop, or restart
  commands.

## Tray And Menu Bar

> This section explains tray and menu-bar integration.

- Apps can opt into tray or menu-bar integration.
  - The config requires a tray icon.
  - The icon is validated in development.
  - The icon is included in package resources.
- Frontend code can ask the tray integration to restore the main window.
- Frontend code can subscribe to tray restore-window events.

## Notifications

> This section describes native notification support and its current limits.

- Cefari prepares native notification support during desktop startup.
  - It uses the configured app identifier.
  - It uses the configured app display name.
  - It does not send notifications on startup.
  - It does not request permission on startup.
- Apps can check notification permission state through the TypeScript API.
- Apps can request notification permission from user-visible flows.
- Apps can send notifications with a title and optional body.
- Apps can subscribe to notification response events.
- Notification requests are validated.
  - Titles must be non-empty.
  - Optional text fields are trimmed.
  - Blank optional text fields are rejected.
- Current notification IPC is not wired end to end in the desktop dispatcher.
  - The TypeScript API exists.
  - The protocol reserves permission and response events.
  - The desktop dispatcher currently reports notification commands as
    unsupported.

## App Data Filesystem

> This section covers app-scoped file access from frontend code.

- Apps can access files inside Cefari's managed app-data directory.
  - Paths are relative to the app-data directory.
  - Absolute paths are rejected.
  - Parent-directory traversal is rejected.
  - Arbitrary OS paths are not exposed.
- Apps can read and write files.
  - Text reads can return strings.
  - Binary reads can return bytes.
  - Text writes use UTF-8 by default.
  - Binary writes cross the IPC boundary as base64.
- Apps can inspect and modify app-data directories.
  - They can list directory entries.
  - They can create directories.
  - They can remove files or directories.
  - They can rename files.
  - They can copy files.
  - They can stat files.
  - They can check whether a path is accessible.
- Apps can use higher-level file helpers.
  - They can get the app-data directory display path.
  - They can read and write text.
  - They can read and write bytes.
  - They can read and write JSON.
  - They can convert app-data files into object URLs for frontend use.
- The app-data filesystem intentionally omits broad OS filesystem features.
  - It does not expose file descriptors.
  - It does not expose streams.
  - It does not expose file watchers.
  - It does not expose config, cache, logs, resources, or update directories.

## Platform And Distribution Support

> This section names the platforms and tools Cefari builds on.

- Cefari targets desktop app distribution on macOS, Linux, and Windows.
- Cefari uses CEF for the embedded browser runtime.
- Cefari uses Vite for frontend development and builds.
- Cefari uses Deno for daemon development and compilation.
- Cefari uses Rust for the native runtime.
- Cefari uses `cargo-packager` for native package generation when available.
- Cefari uses `cargo-codesign`-style tooling for signing, notarization, and
  updater signatures.

## Project Status

> This section states Cefari's current maturity and compatibility stance.

- Cefari is pre-alpha.
- Breaking changes are expected.
- Legacy compatibility is not a product goal.
- The public CLI surface is intentionally small.
- Runtime and tooling boundaries are kept separate so the app runtime stays
  focused.
