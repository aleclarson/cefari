# Namespace APIs

`@cefari/app` exposes task-oriented namespaces through the `cefari` object and
as named exports.

```ts
import { cefari } from "@cefari/app";

await cefari.window.focus();
```

## App

Use `cefari.app` for app-level commands.

```ts
await cefari.app.quit();
```

`tryQuit()` returns a result object instead of throwing:

```ts
const result = await cefari.app.tryQuit();

if (!result.ok) {
  console.error(result.error.message);
}
```

Current methods:

- `quit(): Promise<void>`
- `tryQuit(): Promise<CefariResult<void>>`

## Window

Use `cefari.window` for native window operations:

```ts
await cefari.window.show();
await cefari.window.focus();

const state = await cefari.window.setTitle("Dashboard");
console.log(state.title);
```

`show()`, `focus()`, `close()`, and `setTitle()` return the resulting
`WindowState`.

```ts
const closedState = await cefari.window.close();
console.log(closedState.visible);
```

Current methods:

- `show(): Promise<WindowState>`
- `focus(): Promise<WindowState>`
- `close(): Promise<WindowState>`
- `setTitle(title: string): Promise<WindowState>`
- `onShown(handler): Unsubscribe`
- `onFocused(handler): Unsubscribe`
- `onClosed(handler): Unsubscribe`

## Shell

Use `cefari.shell` for OS shell tasks:

```ts
await cefari.shell.openLogs();

const opened = await cefari.shell.openExternalUrl("https://example.com");
console.log(opened.url);
```

`openExternalUrl` accepts either a string or a `URL`. Rust validates the URL
before opening it.

```ts
await cefari.shell.openExternalUrl(new URL("https://example.com"));
```

`reloadUi()` is a reserved wrapper over the `reloadUi` IPC command. The current
desktop dispatcher returns `unsupported` because CEF UI reload is not wired yet.

```ts
try {
  await cefari.shell.reloadUi();
} catch (error) {
  console.error(error);
}
```

Current methods:

- `openLogs(): Promise<void>`
- `tryOpenLogs(): Promise<CefariResult<void>>`
- `reloadUi(): Promise<void>`
- `openExternalUrl(url: string | URL): Promise<ExternalUrlResult>`

## Updates

Use `cefari.updates` for update state and user-triggered update checks:

```ts
const state = await cefari.updates.state();

switch (state.state) {
  case "notConfigured":
    break;
  case "current":
    break;
  case "available":
    break;
  case "error":
    break;
}
```

Run a check when the user asks for one:

```ts
const result = await cefari.updates.check();

if (result.state === "available") {
  console.log(`update available: ${result.version}`);
}
```

Subscribe to runtime update state changes:

```ts
const unsubscribe = cefari.updates.onStateChanged((state) => {
  console.log(state.state);
});

unsubscribe();
```

Current methods:

- `state(): Promise<UpdateStateResult>`
- `check(): Promise<UpdateCheckResult>`
- `onStateChanged(handler): Unsubscribe`

The IPC bridge does not currently expose a command to download and install an
available update, restart the app, or apply an update and restart in one step.
Rust has update-checking support and a core install helper, but the frontend
bridge only exposes `updateState` and `updateCheck`.

## Service

Use `cefari.service` when the UI needs daemon status:

```ts
const service = await cefari.service.status();
console.log(service.status);
```

Current methods:

- `status(): Promise<ServiceStatusResult>`
- `onStatusChanged(handler): Unsubscribe`

Service lifecycle commands are not exposed through the current TypeScript
wrapper. Add Rust IPC commands first when frontend code needs start, stop, or
restart actions.

## Tray

Subscribe to tray restore events when the UI needs to refresh visible state:

```ts
const unsubscribe = cefari.tray.onRestoreWindow(() => {
  console.log("tray requested window restore");
});

unsubscribe();
```

The wrapper also exposes the command used by Rust tray handling:

```ts
const result = await cefari.tray.restoreWindow();
console.log(result.restored);
```

Current methods:

- `restoreWindow(): Promise<TrayResult>`
- `onRestoreWindow(handler): Unsubscribe`

## Notifications

Notification commands are typed in the protocol and wrapped by `@cefari/app`,
but the current desktop dispatcher returns `unsupported` until notification IPC
is wired end to end.

```ts
const permission = await cefari.notifications.permissionState();

if (permission.allowed) {
  const sent = await cefari.notifications.send({
    title: "Build complete",
    body: "The package is ready.",
  });

  console.log(sent.id);
}
```

Prompt for permission only from an explicit user-visible action:

```ts
button.addEventListener("click", async () => {
  const permission = await cefari.notifications.requestPermission();
  console.log(permission.allowed);
});
```

Handle notification responses:

```ts
const unsubscribe = cefari.notifications.onResponse((event) => {
  console.log(event.id, event.action);
});
```

Current methods:

- `permissionState(): Promise<NotificationPermission>`
- `requestPermission(): Promise<NotificationPermission>`
- `send(input): Promise<NotificationSent>`
- `onResponse(handler): Unsubscribe`
