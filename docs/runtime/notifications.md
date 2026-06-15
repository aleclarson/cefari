# Runtime Notification Boundary

This page is for runtime contributors. App developers should use
[Notification Behavior](../notifications.md) and the
[`cefari/app` notification APIs](../typescript/namespaces.md).

## Ownership

Cefari desktop notifications are routed through `cefari-desktop`, not through
`packages/cefari-cli` or `cefari-core`.

The desktop runtime owns a small Cefari wrapper around `user-notify`. App code
calls the Cefari notification abstraction so permission checks, fallback
behavior, and IPC exposure stay behind one runtime boundary.

## Startup

The desktop runtime creates and registers a notification manager during startup
using the configured app identifier and display name. The manager is held for
the lifetime of the native shell.

Startup does not send notifications and does not request notification
permission.

## Request Model

Runtime notification requests must have a non-empty title. Optional body,
subtitle, thread id, and category id values are trimmed and rejected when blank.

The wrapper checks notification permission before sending. If the OS reports
that notifications are denied or undetermined, Cefari returns a permission
denied outcome instead of treating that state as a crash.

## Dependency Boundary

`user-notify` is a `cefari-desktop` dependency only. Do not add it to
`cefari-core` or `packages/cefari-cli`.

Notification actions, replies, and categories are not treated as a stable
cross-platform Cefari contract yet.
