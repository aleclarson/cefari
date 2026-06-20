# Namespace APIs

`cefari/app` exposes task-oriented namespaces through the `cefari` object and
as named exports.

```ts
import { cefari } from "cefari/app";

await cefari.desktop.window.focus();
await cefari.desktop.windows.create({ id: "settings", route: "/settings" });
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

Use `cefari.desktop.window` for current-window convenience operations:

```ts
await cefari.desktop.window.show();
await cefari.desktop.window.focus();

const state = await cefari.desktop.window.setTitle("Dashboard");
console.log(state.title);
```

`show()`, `focus()`, `close()`, and `setTitle()` return the resulting
`WindowState`. These methods target the current native window by default. They
also accept an optional target when code needs to use the convenience namespace
against a specific window.

```ts
const closedState = await cefari.desktop.window.close();
console.log(closedState.visible);
```

Current methods:

- `current(): Promise<WindowState>`
- `list(): Promise<WindowState[]>`
- `create(options?: WindowCreateOptions): Promise<WindowState>`
- `show(): Promise<WindowState>`
- `focus(): Promise<WindowState>`
- `close(): Promise<WindowState>`
- `setTitle(title: string): Promise<WindowState>`
- `onShown(handler): Unsubscribe`
- `onFocused(handler): Unsubscribe`
- `onClosed(handler): Unsubscribe`

## Windows

Use `cefari.desktop.windows` when code needs to create or target specific native
windows:

```ts
const settings = await cefari.desktop.windows.create({
  id: "settings",
  route: "/settings",
  title: "Settings",
  width: 720,
  height: 560,
  persistKey: "settings",
});

await cefari.desktop.windows.focus(settings.id);
await cefari.desktop.windows.setTitle("settings", "Preferences");
```

The startup window is always `main`. Secondary windows load trusted app
frontend content: Vite dev routes in development and `cefari://app/index.html`
route metadata in packaged mode. Cefari persists geometry for `main` by
default and for secondary windows only when `persistKey` is supplied.

Parented and modal windows are supported for secondary windows:

```ts
await cefari.desktop.windows.create({
  id: "dialog",
  route: "/dialog",
  parentId: "main",
  modal: true,
});
```

Modal windows require a valid parent. Closing a parent closes its child
windows. Native modal behavior varies by platform; Cefari still tracks
`parentId` and `modal` in `WindowState`.

Current methods:

- `current(): Promise<WindowState>`
- `list(): Promise<WindowState[]>`
- `get(target): Promise<WindowState | undefined>`
- `create(options?: WindowCreateOptions): Promise<WindowState>`
- `show(target): Promise<WindowState>`
- `focus(target): Promise<WindowState>`
- `close(target): Promise<WindowState>`
- `setTitle(target, title): Promise<WindowState>`
- `onCreated(handler, filter?): Unsubscribe`
- `onShown(handler, filter?): Unsubscribe`
- `onFocused(handler, filter?): Unsubscribe`
- `onBlurred(handler, filter?): Unsubscribe`
- `onCloseRequested(handler, filter?): Unsubscribe`
- `onClosed(handler, filter?): Unsubscribe`
- `onMoved(handler, filter?): Unsubscribe`
- `onResized(handler, filter?): Unsubscribe`
- `onTitleChanged(handler, filter?): Unsubscribe`

## Workers

Use `cefari.workers` to spawn configured Deno script workers from trusted
frontend code:

```ts
import { cefari } from "cefari/app";

const worker = await cefari.workers.spawn("thumbnailer", {
  cacheDir: "cache/thumbnails",
});

const result = await worker.invoke("thumbnail", {
  inputPath: "uploads/photo.jpg",
});

const unsubscribe = worker.onMessage((message) => {
  console.log(message);
});
```

Worker names, init input, method names, method inputs, method messages, and
method outputs come from the generated `.cefari/workers.d.ts` registry. A
worker must be listed in `cefari.config.ts` before frontend code can spawn it.

In development, workers run as Deno source scripts with the permissions
configured for that worker. In packaged apps, workers run as compiled
executables produced during `cefari build`. They are separate from the app
daemon and have their own process lifecycle.

`worker.invoke()` targets one concrete worker process instance. This keeps
multiple instances of the same configured worker unambiguous.

`worker.onMessage()` receives messages posted with `context.postMessage()`. Use
`onExit()` and `onError()` to observe lifecycle and protocol failures.

Use `cefari.workers.run()` for explicit one-shot work. It spawns a worker,
invokes one method, and terminates the handle.

Current methods:

- `spawn(name, init): Promise<CefariWorkerHandle>`
- `run(name, init, method, input): Promise<output>`
- `terminate(id): Promise<void>`
- `list(): Promise<WorkerState[]>`

Current handle methods:

- `invoke(method, input): Promise<output>`
- `terminate(): Promise<void>`
- `onMessage(handler): Unsubscribe`
- `onExit(handler): Unsubscribe`
- `onError(handler): Unsubscribe`

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

Use the browser's own reload API for frontend reloads:

```ts
window.location.reload();
```

Current methods:

- `openLogs(): Promise<void>`
- `tryOpenLogs(): Promise<CefariResult<void>>`
- `openExternalUrl(url: string | URL): Promise<ExternalUrlResult>`

## Logs

Use `cefari.logs` for structured frontend logs:

```ts
await cefari.logs.info("settings.saved", {
  panel: "notifications",
});

await cefari.logs.warn("sync.delayed", {
  retryAfterMs: 5000,
});
```

Log properties are stored as JSON in Cefari's local SQLite log database.
Secret-like property values are redacted before persistence.

Use `tryWrite()` when the UI wants a result object instead of a thrown error:

```ts
const result = await cefari.logs.tryWrite({
  level: "info",
  message: "export.finished",
  properties: { count: 12 },
});
```

Current methods:

- `debug(message, properties?): Promise<void>`
- `info(message, properties?): Promise<void>`
- `log(message, properties?): Promise<void>`
- `warn(message, properties?): Promise<void>`
- `error(message, properties?): Promise<void>`
- `write(entry): Promise<void>`
- `tryWrite(entry): Promise<CefariResult<void>>`

## Updates

Use `cefari.desktop.updates` for update state and user-triggered update checks:

```ts
const state = await cefari.desktop.updates.state();

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
const result = await cefari.desktop.updates.check();

if (result.state === "available") {
  console.log(`update available: ${result.version}`);
}
```

Apply the checked update from an explicit user-visible action:

```ts
const checked = await cefari.desktop.updates.check();

if (checked.state === "available") {
  const applied = await cefari.desktop.updates.apply({
    updateId: checked.updateId,
  });

  if (applied.restartRequired) {
    await cefari.desktop.updates.restart();
  }
}
```

For a one-shot action, use `applyAndRestart()`:

```ts
await cefari.desktop.updates.applyAndRestart({ updateId: result.updateId });
```

`apply()` installs the native update that Rust cached from the most recent
successful `check()`. Frontend code never passes an update URL or signature.
Some platform installers can terminate or relaunch the current process during
installation, so app code must not rely on `apply()` always returning.

Subscribe to runtime update state changes:

```ts
const unsubscribe = cefari.desktop.updates.onStateChanged((state) => {
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

Use `cefari.desktop.service` when the UI needs daemon status:

```ts
const service = await cefari.desktop.service.status();
console.log(service.status);
```

Current methods:

- `status(): Promise<ServiceStatusResult>`
- `onStatusChanged(handler): Unsubscribe`

Service lifecycle commands are not exposed through the current TypeScript
wrapper. Add Rust IPC commands first when frontend code needs start, stop, or
restart actions.

## Daemon

Use `cefari.desktop.daemon` when a configured daemon needs a low-level byte stream:

```ts
const connection = await cefari.desktop.daemon.connect();
const writer = connection.writable.getWriter();
await writer.write(new TextEncoder().encode("ping"));
```

Current methods:

- `isConfigured(): boolean`
- `connect(): Promise<DaemonConnection>`

`DaemonConnection.readable` carries daemon-to-webview bytes.
`DaemonConnection.writable` carries webview-to-daemon bytes. If the app omits
`daemon` from `cefari.config.ts`, `connect()` rejects with a typed unsupported
error.

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

if (await cefari.files.exists("assets/icon.png")) {
  console.log("icon is cached");
}

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
- `exists(path): Promise<boolean>`
- `readJson(path): Promise<JsonValue>`
- `writeJson(path, value, options?): Promise<void>`
- `toObjectUrl(path, options?): Promise<string>`

## Tray

Subscribe to tray restore events when the UI needs to refresh visible state:

```ts
const unsubscribe = cefari.desktop.tray.onRestoreWindow(() => {
  console.log("tray requested window restore");
});

unsubscribe();
```

The wrapper also exposes the command used by Rust tray handling:

```ts
const result = await cefari.desktop.tray.restoreWindow();
console.log(result.restored);
```

Current methods:

- `restoreWindow(): Promise<TrayResult>`
- `onRestoreWindow(handler): Unsubscribe`

## Downloads

Use `cefari.downloads` to control browser-initiated downloads that the native
runtime has already started:

```ts
const unsubscribe = cefari.on("download.completed", async (download) => {
  await cefari.downloads.reveal(download.id);
});
```

CEF downloads are native-runtime owned. The runtime validates the download URL,
shows the OS save dialog, emits lifecycle events, and writes only after the user
chooses a destination.

Current methods:

- `cancel(id: string): Promise<DownloadResult>`
- `reveal(id: string): Promise<DownloadResult>`

## Notifications

Notification commands are typed in the protocol, wrapped by `cefari/app`, and
dispatched by the desktop runtime.

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

Default notification clicks emit a response event and focus the main window.
Dismiss responses emit an event without focusing the main window.

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
