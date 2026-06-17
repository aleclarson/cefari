# Namespace APIs

`cefari/app` exposes task-oriented namespaces through the `cefari` object and
as named exports.

```ts
import { cefari } from "cefari/app";

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
  case "checking":
    break;
  case "available":
    break;
  case "applying":
    break;
  case "readyToRestart":
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

Apply the checked update from an explicit user-visible action:

```ts
const checked = await cefari.updates.check();

if (checked.state === "available") {
  const applied = await cefari.updates.apply({
    updateId: checked.updateId,
  });

  if (applied.restartRequired) {
    await cefari.updates.restart();
  }
}
```

For a one-shot action, use `applyAndRestart()`:

```ts
await cefari.updates.applyAndRestart({ updateId: result.updateId });
```

`apply()` installs the native update that Rust cached from the most recent
successful `check()`. Frontend code never passes an update URL or signature.
Some platform installers can terminate or relaunch the current process during
installation, so app code must not rely on `apply()` always returning.

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
- `apply(options?: UpdateApplyOptions): Promise<UpdateApplyResult>`
- `restart(): Promise<void>`
- `applyAndRestart(options?: UpdateApplyOptions): Promise<void>`
- `onStateChanged(handler): Unsubscribe`

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

## App Data Files

Use `cefari.fs` for files inside Cefari's managed app-data directory. Paths are
relative to that directory. Absolute paths and `..` traversal are rejected by
Rust before filesystem access.

```ts
import { cefari } from "cefari/app";

await cefari.fs.writeFile("settings/preferences.json", "{\"theme\":\"dark\"}");

const preferences = await cefari.fs.readFile(
  "settings/preferences.json",
  "utf8",
);
console.log(preferences);
```

The API mirrors the async `node:fs` shape where Cefari supports the operation:

```ts
await cefari.fs.mkdir("cache/images", { recursive: true });
await cefari.fs.copyFile("cache/source.png", "cache/images/source.png");

const entries = await cefari.fs.readdir("cache", { withFileTypes: true });
for (const entry of entries) {
  if (entry.isDirectory()) console.log(entry.path);
}

const stat = await cefari.fs.stat("cache/images/source.png");
console.log(stat.size, stat.isFile());

await cefari.fs.rm("cache/images", { recursive: true, force: true });
```

`readFile(path)` returns `Uint8Array`. Pass `"utf8"` or `{ encoding: "utf8" }`
when app code expects text. `writeFile` accepts strings, `Uint8Array`, and
`ArrayBuffer`; string writes default to UTF-8 text and byte writes are encoded
as base64 over the IPC boundary.

Use `cefari.files` for app-oriented helpers:

```ts
const root = await cefari.files.appDataDir();
console.log(root.displayPath);

const state = await cefari.files.readJson("state.json");
await cefari.files.writeJson("state.json", { ...state, openedAt: Date.now() });

const iconUrl = await cefari.files.toObjectUrl("assets/icon.png", {
  type: "image/png",
});
```

This filesystem API is scoped to app data. It does not expose file descriptors,
streams, watchers, arbitrary OS paths, or direct access to config, cache, logs,
resources, or update directories.

Current `cefari.fs` methods:

- `readFile(path, options?): Promise<string | Uint8Array>`
- `writeFile(path, data, options?): Promise<void>`
- `readdir(path?, options?): Promise<string[] | CefariDirent[]>`
- `mkdir(path, options?): Promise<void>`
- `rm(path, options?): Promise<void>`
- `rename(from, to): Promise<void>`
- `copyFile(from, to): Promise<void>`
- `stat(path): Promise<CefariStats>`
- `access(path): Promise<boolean>`

Current `cefari.files` methods:

- `appDataDir(): Promise<AppDataDir>`
- `readText(path): Promise<string>`
- `writeText(path, contents): Promise<void>`
- `readBytes(path): Promise<Uint8Array>`
- `writeBytes(path, contents): Promise<void>`
- `readJson(path): Promise<JsonValue>`
- `writeJson(path, value, options?): Promise<void>`
- `toObjectUrl(path, options?): Promise<string>`

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

Notification commands are typed in the protocol and wrapped by `cefari/app`,
but the current desktop dispatcher returns `unsupported` until notification IPC
is wired end to end.

```ts
const capabilities = await cefari.notifications.capabilities();
const permission = await cefari.notifications.permissionState();

if (permission.allowed) {
  const sent = await cefari.notifications.send({
    title: "Build complete",
    body: "The package is ready.",
    subtitle: "Release",
    image: { source: "appResource", path: "images/build.png" },
    icon: { source: "appData", path: "icons/build.png" },
    userInfo: { buildId: "123" },
  });

  console.log(sent.id);
}
```

Register native categories and actions before sending notifications that use
them:

```ts
await cefari.notifications.registerCategories([
  {
    id: "message",
    actions: [
      { type: "action", id: "open", title: "Open" },
      {
        type: "textInput",
        id: "reply",
        title: "Reply",
        inputButtonTitle: "Send",
        inputPlaceholder: "Message",
      },
    ],
  },
]);
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
  console.log(event.id, event.action, event.userText, event.userInfo);
});
```

Current methods:

- `permissionState(): Promise<NotificationPermission>`
- `requestPermission(): Promise<NotificationPermission>`
- `capabilities(): Promise<NotificationCapabilities>`
- `registerCategories(categories): Promise<NotificationCategoriesRegistered>`
- `send(input): Promise<NotificationSent>`
- `active(): Promise<ActiveNotification[]>`
- `removeDelivered(ids): Promise<NotificationRemoved>`
- `removeAllDelivered(): Promise<NotificationRemoved>`
- `onResponse(handler): Unsubscribe`
