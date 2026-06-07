# Native Capabilities

Cefari routes native desktop behavior through Rust runtime code, not through ad hoc frontend calls.

## Rust-Owned Areas

The desktop runtime owns:

- window lifecycle actions
- native menu actions
- tray/menu-bar actions
- opening validated external URLs
- opening the runtime log location
- update checks and installs
- daemon service status and lifecycle helpers
- OS notification setup

Shared runtime helpers live in `cefari-core`; native shell wiring lives in `cefari-desktop`.

## IPC Contract

Cefari's typed IPC payloads are defined in Rust and exported to TypeScript with Specta. See [Cefari IPC Protocol](../ipc.md).

Frontend code should use the generated TypeScript types and the Cefari bridge wrapper instead of inventing stringly typed native commands.

## Notifications

Notification delivery is owned by `cefari-desktop`. Startup registers the notification manager but does not send notifications or request permission. See [Desktop Notifications](../notifications.md).

## Custom Titlebars

Use Cefari's CSS contract for drag regions. See [Cefari CSS Contract](../css-contract.md).
