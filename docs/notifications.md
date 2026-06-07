# Desktop Notifications

Cefari desktop notifications are routed through `cefari-desktop`, not through `cefari-cli` or `cefari-core`.

The desktop crate owns a small Cefari wrapper around `user-notify`. App code should call the Cefari notification abstraction rather than constructing `user-notify` requests directly. That keeps permission checks, fallback behavior, and IPC exposure behind one runtime boundary.

## Startup Behavior

The desktop runtime creates and registers a notification manager during startup using the configured app identifier and display name.

Startup does not send a notification and does not request notification permission. Permission prompts must be triggered by an explicit user-visible flow.

## Request Contract

Notification requests must have a non-empty title. Optional body, subtitle, thread id, and category id values are trimmed and rejected when blank.

The wrapper checks notification permission before sending. If the OS reports that notifications are denied or undetermined, Cefari returns `PermissionDenied` instead of treating that state as a crash.

## Platform Notes

- macOS requires a real app bundle identifier before native notifications can be delivered. Development or unbundled runs may use the crate's mock fallback.
- macOS permission APIs must run on the main thread. Cefari keeps permission prompts as explicit calls so startup does not surprise users.
- Windows toast setup uses the configured app identifier. If toast notifier creation fails, the crate falls back to a mock manager.
- Linux and other XDG desktop targets use the desktop notification service exposed by the session. Availability depends on the user's desktop environment.
- Notification actions, replies, and categories are not treated as a cross-platform Cefari contract yet.

## Dependency Boundary

`user-notify` is a `cefari-desktop` dependency only. It must not be added to `cefari-core` or `cefari-cli`.

See [Native Capabilities](guides/native-capabilities.md) for the broader Rust-owned desktop surface.
