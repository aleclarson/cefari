# Events And Errors

`cefari/app` exposes typed event helpers and typed errors over the generated
IPC protocol.

## Typed Events

Use typed event names for normal app code:

```ts
import { cefari } from "cefari/app";

const unsubscribe = cefari.on("windowFocused", (state) => {
  console.log(state.focused);
});

unsubscribe();
```

Available event names:

- `windowShown`
- `windowFocused`
- `windowClosed`
- `trayRestoreWindow`
- `updateStateChanged`
- `serviceStatusChanged`
- `notification.response`

Namespace helpers call the same event system:

```ts
const offFocus = cefari.window.onFocused((state) => {
  console.log(state.title);
});

const offUpdate = cefari.updates.onStateChanged((state) => {
  console.log(state.state);
});

offFocus();
offUpdate();
```

Use `onAnyEvent` only for logging, diagnostics, or bridge tooling:

```ts
const unsubscribe = cefari.onAnyEvent((event) => {
  console.debug(event);
});
```

Outside the desktop shell, event subscriptions are no-ops and return an
unsubscribe function.

## Thrown Errors

Wrapper methods throw `CefariError` when Rust returns a typed IPC error or when
the native bridge is unavailable:

```ts
import { cefari, isCefariError } from "cefari/app";

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

`CefariError` exposes:

- `name`: always `CefariError`
- `code`: the IPC error code
- `details`: the generated IPC error object
- `command`: the command associated with the failure when known
- `message`: a human-readable message derived from the IPC error

## Result-Style Calls

Use `tryInvoke` when the UI wants result objects instead of thrown errors:

```ts
const result = await cefari.tryInvoke({ command: "updateState" });

if (result.ok) {
  console.log(result.value);
} else {
  console.error(result.error.code);
}
```

Some namespaces expose result-style helpers for common failure-prone commands:

```ts
const quit = await cefari.app.tryQuit();
const logs = await cefari.shell.tryOpenLogs();
```

The result shape is:

```ts
type CefariResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: CefariError };
```
