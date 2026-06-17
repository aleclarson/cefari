# Native Capabilities

Cefari routes native desktop behavior through Rust runtime code, not through ad
hoc frontend calls.

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

## TypeScript APIs

Frontend code should use `cefari/app` for native desktop capabilities instead
of inventing stringly typed native commands. See the
[TypeScript App Guide](../typescript/index.md).

## Notifications

Notification delivery is owned by the desktop runtime. Startup prepares
notification support but does not send notifications or request permission. See
[Notification Behavior](../notifications.md).

The notification capability includes permission checks, permission prompts where
the OS supports them, rich delivery fields, category/action registration,
inline reply payloads where supported, response events, active notification
listing, delivered notification removal, and packaged activation metadata.
Support for each field is platform-dependent; the platform matrix in
[Notification Behavior](../notifications.md) is the source of truth.

## Custom Titlebars

Use Cefari's CSS contract for drag regions. See
[Cefari CSS Contract](../css-contract.md).
