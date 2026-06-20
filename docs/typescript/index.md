# TypeScript Overview

Use `cefari/app` from frontend TypeScript, TSX, JSX, or JavaScript code when
the app needs native Cefari behavior. The package wraps `window.cefari` with
typed, task-oriented functions and re-exports the Specta-generated IPC types.

The package is intended for app UI code that runs inside the Cefari desktop
shell. Browser preview, ordinary web tests, and standalone Vite pages do not
have the native bridge unless a test installs a mock bridge.

## Package Boundary

The package exports six public TypeScript entrypoints:

- `cefari/app`: ergonomic wrappers, namespace APIs, bridge helpers, event
  helpers, error helpers, and generated IPC types.
- `cefari/ipc`: generated IPC types only.
- `cefari/daemon`: daemon-side stdio helpers for configured daemon programs.
- `cefari/logs`: Deno-side structured log store and logger helpers.
- `cefari/logs/sentry`: an official Sentry sink for exported Cefari log rows.
- `cefari/worker`: helpers for Deno worker entry scripts.

The default app object is `cefari`:

```ts
import { cefari } from "cefari/app";

if (cefari.isAvailable()) {
  await cefari.desktop.window.setTitle("Dashboard");
  await cefari.desktop.windows.create({ id: "settings", route: "/settings" });
}
```

Named namespace exports are also available when code only needs one surface:

```ts
import { shell, updates } from "cefari/app";

await shell.openExternalUrl("https://example.com");
const state = await updates.state();
```

Configured daemon streams are available through `cefari.desktop.daemon`:

```ts
import { cefari } from "cefari/app";

if (cefari.desktop.daemon.isConfigured()) {
  const connection = await cefari.desktop.daemon.connect();
  const writer = connection.writable.getWriter();
  await writer.write(new TextEncoder().encode("ping"));
  await writer.close();
  await connection.close();
}
```

`connection.writable` sends bytes from the webview to the daemon.
`connection.readable` receives bytes from the daemon. If the app omits the
`daemon` config section, `connect()` rejects with a typed unsupported
`CefariError`.

Daemon programs can use `cefari/daemon`:

```ts
import { connect, isCefariDaemon } from "cefari/daemon";

if (isCefariDaemon()) {
  const connection = connect();
  await connection.readable.pipeTo(connection.writable);
}
```

The v1 daemon stream transport is stdio. The public API is transport-agnostic
and does not expose HTTP or WebSocket selection.

Daemon and worker programs can use `cefari/logs` when they need structured log
rows in the same local database:

```ts
import { createLogger } from "cefari/logs";

const logger = createLogger({ scope: "daemon" });
logger.info("daemon.ready", { port: 53117 });
```

Operator tooling can use `cefari/logs/sentry` when it needs to send exported
rows to Sentry without wiring separate source-specific hooks:

```ts
import { createLogStore } from "cefari/logs";
import { createSentryLogSink } from "cefari/logs/sentry";

const store = createLogStore();
const sink = createSentryLogSink({
  dsn: process.env.SENTRY_DSN ?? "",
  environment: "production",
  release: "my-app@1.2.3",
});

const records = store.exportBatch({ exporter: "sentry", limit: 100 });
await sink.export(records);
if (await sink.flush()) {
  store.ackExport("sentry", records.at(-1)?.id ?? 0);
}
```

Most apps should use `cefari logs export sentry`; direct adapter use is for
custom operator processes.

Generated IPC types are re-exported for tools, tests, and bridge code:

```ts
import type { CefariIpcCommand, CefariIpcResult } from "cefari/app";
```

## Add The Package

In this repository, the Vite React template maps `cefari/app` to the local
package source:

```json
{
  "imports": {
    "cefari/app": "../../../npm/src/app/mod.ts",
    "cefari/daemon": "../../../npm/src/daemon.ts"
  }
}
```

Template builds also add a Vite alias for the same path because Vite does not
read Deno import maps during bundling.

Generated apps should import from `cefari/app` after the umbrella `cefari`
package is installed.

## Check Bridge Availability

Use `cefari.isAvailable()` before rendering native-only controls:

```ts
import { cefari } from "cefari/app";

if (cefari.isAvailable()) {
  console.log("running inside the Cefari desktop shell");
}
```

Calling wrappers outside the desktop shell rejects with a typed `CefariError`
whose code is `unsupported`.

## Documentation Map

- [Namespace APIs](namespaces.md): `app`, `window`, `windows`, `workers`,
  `shell`, `logs`, `updates`, `service`, `tray`, `notifications`, `daemon`,
  `fs`, and `files`.
- [Events And Errors](events-and-errors.md): typed event subscriptions,
  low-level event logging, thrown errors, and result-style helpers.
