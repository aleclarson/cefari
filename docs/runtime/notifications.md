# Runtime Notification Boundary

This page is for runtime contributors. App developers should use
[Notification Behavior](../notifications.md) and the
[`cefari/app` notification APIs](../typescript/namespaces.md).

## Ownership

Cefari desktop notifications are routed through `cefari-desktop`, not through
`npm` or `cefari-core`.

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
subtitle, thread id, category id, action labels, reply labels, and media paths
are trimmed and rejected when blank.

The IPC contract mirrors `user-notify`'s practical feature set: subtitle,
image, icon, rounded icon, thread id, category id, XDG category, user info,
category registration, action buttons, inline replies, active notification
listing, and delivered notification removal.

Notification media crosses IPC as Cefari-owned app-resource or app-data
references. `cefari-desktop` resolves those references before passing paths to
`user-notify`.

The wrapper checks notification permission before sending. If the OS reports
that notifications are denied or undetermined, Cefari returns a permission
denied outcome instead of treating that state as a crash.

## Dependency Boundary

`user-notify` is a `cefari-desktop` dependency only. Do not add it to
`cefari-core` or `npm`.

`cefari-core` owns only serializable IPC types. `npm` owns only typed frontend
wrappers. Platform-specific notification behavior stays inside
`cefari-desktop`.
