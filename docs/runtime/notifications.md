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

The desktop runtime creates a notification manager during startup using the
configured app identifier and display name. The manager is held for the
lifetime of the native shell.

After the Tao event loop is available, Cefari attaches a response sink so
`user-notify` callbacks cross into the UI thread before touching CEF. The
manager registers the native response handler before the first notification is
sent, or when categories are explicitly registered.

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

## Response Events

`user-notify` response callbacks are converted into `notification.response`
IPC events. The desktop event loop serializes those events and invokes
`window.__CEFARI_IPC_EVENT__` in the main CEF frame.

Default notification clicks emit the response event and then show/focus the
main window if it still exists. Dismiss responses only emit the event.

Because `user-notify` registers its native response handler once, apps should
register categories before sending notifications that use category actions.

Live smoke coverage can verify the native-to-frontend bridge without showing an
OS notification by injecting a synthetic `notification.response` IPC event
through the CEF event helper after the main frame has loaded.

## Dependency Boundary

`user-notify` is a `cefari-desktop` dependency only. Do not add it to
`cefari-core` or `npm`.

`cefari-core` owns only serializable IPC types. `npm` owns only typed frontend
wrappers. Platform-specific notification behavior stays inside
`cefari-desktop`.
