# TypeScript App Guide

Use `@cefari/app` from frontend TypeScript or JSX code when the app needs native
Cefari behavior. The package wraps `window.cefari` with typed, task-oriented
functions and re-exports the Specta-generated IPC types.

## Add The Package

In this repository, the Vite React template maps `@cefari/app` to the local
package source:

```json
{
  "imports": {
    "@cefari/app": "../../../packages/cefari-app/src/mod.ts"
  }
}
```

Template builds also add a Vite alias for the same path because Vite does not
read Deno import maps during bundling.

Generated apps should import from the installed package location used by their
package manager once `@cefari/app` is published or vendored into the app.

## Check Whether Cefari Is Available

Browser preview and ordinary web tests do not have the native bridge. Use
`cefari.isAvailable()` before showing native-only controls:

```ts
import { cefari } from "@cefari/app";

if (cefari.isAvailable()) {
  console.log("running inside the Cefari desktop shell");
}
```

Calling wrappers outside the desktop shell rejects with a typed `CefariError`
whose code is `unsupported`.

## Read Update State

Use `cefari.updates.state()` for the current runtime update state:

```ts
import { cefari } from "@cefari/app";

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

## Manage The Window

Use the window namespace for native window operations:

```ts
await cefari.window.show();
await cefari.window.focus();

const state = await cefari.window.setTitle("Dashboard");
console.log(state.title);
```

`close()` and `setTitle()` return the resulting `WindowState`, matching the Rust
dispatcher contract:

```ts
const closedState = await cefari.window.close();
console.log(closedState.visible);
```

## Open Logs And External URLs

Use the shell namespace for OS shell tasks:

```ts
await cefari.shell.openLogs();

const opened = await cefari.shell.openExternalUrl("https://example.com");
console.log(opened.url);
```

`openExternalUrl` accepts either a string or a `URL`. Rust validates the URL
before opening it.

## Query The Daemon Service

Use the service namespace when the UI needs daemon status:

```ts
const service = await cefari.service.status();
console.log(service.status);
```

Service lifecycle commands are not exposed through the current TypeScript
wrapper. Add Rust IPC commands first when frontend code needs new service
actions.

## Respond To Tray Events

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

## Send Notifications

Notification commands are typed now, but the Rust dispatcher may return
`unsupported` until notification IPC is wired end to end.

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

## Subscribe To Events

Use typed event names for normal app code:

```ts
const unsubscribe = cefari.on("windowFocused", (state) => {
  console.log(state.focused);
});
```

Available event names:

- `windowShown`
- `windowFocused`
- `windowClosed`
- `trayRestoreWindow`
- `updateStateChanged`
- `serviceStatusChanged`
- `notification.response`

Use `onAnyEvent` only for logging, diagnostics, or bridge tooling:

```ts
const unsubscribe = cefari.onAnyEvent((event) => {
  console.debug(event);
});
```

## Handle Errors

Wrapper methods throw `CefariError` when Rust returns a typed IPC error or when
the native bridge is unavailable:

```ts
import { cefari, isCefariError } from "@cefari/app";

try {
  await cefari.shell.openLogs();
} catch (error) {
  if (isCefariError(error)) {
    switch (error.code) {
      case "unsupported":
      case "denied":
      case "invalidCommand":
      case "unknownCommand":
        console.error(error.message);
        break;
    }
  }
}
```

Use `tryInvoke` when the UI wants result objects instead of thrown errors:

```ts
const result = await cefari.tryInvoke({ command: "updateState" });

if (result.ok) {
  console.log(result.value);
} else {
  console.error(result.error.code);
}
```

## Use Raw IPC Only At Boundaries

Prefer namespace wrappers for app code. Use raw `invoke` only when building a
higher-level helper or testing a newly added command before adding a wrapper:

```ts
const result = await cefari.invoke({
  command: "openExternalUrl",
  payload: { url: "https://example.com" },
});
```

Raw calls still unwrap the response and throw `CefariError` on failure.

## Keep Types In Sync

`@cefari/app/src/ipc.ts` is copied from `crates/cefari-core/bindings/ipc.ts`.
When Rust IPC types change:

1. Regenerate `crates/cefari-core/bindings/ipc.ts`.
2. Copy it to `packages/cefari-app/src/ipc.ts`.
3. Update wrapper functions for new commands or result variants.
4. Run package and template checks.

```bash
deno task --cwd packages/cefari-app check
deno task --cwd packages/cefari-app test
deno task --cwd templates/vite-react-basic/frontend check
```
