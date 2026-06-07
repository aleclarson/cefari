# TypeScript Overview

Use `@cefari/app` from frontend TypeScript, TSX, JSX, or JavaScript code when
the app needs native Cefari behavior. The package wraps `window.cefari` with
typed, task-oriented functions and re-exports the Specta-generated IPC types.

The package is intended for app UI code that runs inside the Cefari desktop
shell. Browser preview, ordinary web tests, and standalone Vite pages do not
have the native bridge unless a test installs a mock bridge.

## Package Boundary

`@cefari/app` exports two public entrypoints:

- `@cefari/app`: ergonomic wrappers, namespace APIs, bridge helpers, event
  helpers, error helpers, and generated IPC types.
- `@cefari/app/ipc`: generated IPC types only.

The default app object is `cefari`:

```ts
import { cefari } from "@cefari/app";

if (cefari.isAvailable()) {
  await cefari.window.setTitle("Dashboard");
}
```

Named namespace exports are also available when code only needs one surface:

```ts
import { shell, updates } from "@cefari/app";

await shell.openExternalUrl("https://example.com");
const state = await updates.state();
```

Generated IPC types are re-exported for tools, tests, and bridge code:

```ts
import type { CefariIpcCommand, CefariIpcResult } from "@cefari/app";
```

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

## Check Bridge Availability

Use `cefari.isAvailable()` before rendering native-only controls:

```ts
import { cefari } from "@cefari/app";

if (cefari.isAvailable()) {
  console.log("running inside the Cefari desktop shell");
}
```

Calling wrappers outside the desktop shell rejects with a typed `CefariError`
whose code is `unsupported`.

## Documentation Map

- [Namespace APIs](namespaces.md): `app`, `window`, `shell`, `updates`,
  `service`, `tray`, and `notifications`.
- [Events And Errors](events-and-errors.md): typed event subscriptions,
  low-level event logging, thrown errors, and result-style helpers.
- [Raw IPC And Type Sync](raw-ipc.md): raw `invoke`, generated types, bridge
  shape, reserved commands, and Rust-to-TypeScript sync.

## Source Of Truth

The TypeScript wrapper source owns API behavior:

- `packages/cefari-app/src/mod.ts` owns public exports.
- `packages/cefari-app/src/*.ts` owns namespace wrapper behavior.
- `crates/cefari-core/src/ipc.rs` owns the IPC protocol.
- `crates/cefari-core/bindings/ipc.ts` and
  `packages/cefari-app/src/ipc.ts` own generated TypeScript IPC types.

Do not copy exact generated type signatures into prose docs unless a user needs
to understand a stable concept. Prefer importing generated types in examples so
compiler checks catch drift.
