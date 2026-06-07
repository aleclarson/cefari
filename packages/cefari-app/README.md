# @cefari/app

`@cefari/app` provides ergonomic TypeScript wrappers over Cefari's typed
`window.cefari` bridge.

For task-oriented frontend usage, see the
[TypeScript App Guide](../../docs/typescript/index.md).

The package re-exports the Specta-generated IPC types from Rust and layers
promise-based namespaces on top:

```ts
import { cefari } from "@cefari/app";

const updateState = await cefari.updates.state();
await cefari.window.setTitle("Dashboard");
await cefari.shell.openExternalUrl("https://example.com");
await cefari.files.writeJson("state.json", { ready: true });
```

## API Shape

- `cefari.isAvailable()` reports whether the native bridge exists.
- `cefari.invoke(command)` unwraps a raw IPC command and throws `CefariError` on
  failure.
- `cefari.tryInvoke(command)` returns `{ ok, value }` or `{ ok, error }`.
- `cefari.on(name, handler)` subscribes to typed events.
- `cefari.onAnyEvent(handler)` subscribes to raw generated IPC events.

Namespaces:

- `cefari.app`
- `cefari.window`
- `cefari.shell`
- `cefari.updates`
- `cefari.service`
- `cefari.tray`
- `cefari.notifications`
- `cefari.fs`, a sandboxed async filesystem for the app-data directory
- `cefari.files`, app-data helpers for text, bytes, JSON, object URLs, and root
  discovery

File paths are relative to Cefari's managed app-data directory. The Rust side
rejects absolute paths and parent traversal before filesystem access.

## Contract Source

`src/ipc.ts` is copied from `crates/cefari-core/bindings/ipc.ts`. Keep those
files identical so wrapper drift is caught by TypeScript and the repository
verification checks.

## Local Checks

```bash
deno task check
deno task test
```
