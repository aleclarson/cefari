# Raw IPC And Type Sync

Prefer namespace wrappers for app code. Use raw IPC only when building a
higher-level helper, testing a newly added command before adding a wrapper, or
working on bridge tooling.

## Invoke Raw Commands

Raw `invoke` accepts a generated `CefariIpcCommand`, unwraps the response, and
throws `CefariError` on failure:

```ts
import { cefari } from "cefari/app";

const result = await cefari.invoke({
  command: "openExternalUrl",
  payload: { url: "https://example.com" },
});

console.log(result);
```

Use `tryInvoke` for result-style raw IPC:

```ts
const result = await cefari.tryInvoke({ command: "serviceStatus" });

if (result.ok && result.value.result === "serviceStatus") {
  console.log(result.value.payload.status);
}
```

## Bridge Shape

The desktop runtime installs `window.cefari` for trusted packaged app origins
and allowed localhost development origins.

The TypeScript bridge shape is:

```ts
type CefariBridge = {
  invoke(command: CefariIpcCommand): Promise<CefariIpcResponse>;
  on(handler: (event: CefariIpcEvent) => void): Unsubscribe;
};
```

Tests may install a mock bridge on `globalThis.cefari` or `window.cefari`.
Production app code should treat the bridge as runtime-provided and use
`cefari.isAvailable()` to check for it.

Native events use the same bridge. For example, an opened configured deep link
arrives as:

```ts
{ event: "deepLinkOpened", payload: { url: "myapp://open/item" } }
```

Daemon byte streams do not use generated IPC bindings. Use
`cefari.daemon.connect()` from `cefari/app` for webview-to-daemon and
daemon-to-webview bytes. The stream bridge is intentionally low-level and
separate from `CefariIpcCommand` so apps can layer their own framing or RPC
library on top.

## Current Command Surface

The generated command union currently includes:

- `appQuit`
- `windowShow`
- `windowFocus`
- `windowClose`
- `windowSetTitle`
- `openLogs`
- `reloadUi`
- `openExternalUrl`
- `updateState`
- `updateCheck`
- `updateApply`
- `updateRestart`
- `serviceStatus`
- `trayRestoreWindow`
- `notification`
- `files`

Daemon stream connect/write/close traffic is intentionally not part of this
generated command union.

Some generated commands are reserved for native shell integrations instead of
the `cefari/app` namespace APIs. At the moment:

- `reloadUi` is used by native shell controls. App frontend code should use
  `window.location.reload()` directly.
- `notification` commands are typed, wrapped, and dispatched by the desktop
  runtime. Individual operations can still report `unsupported` when the native
  notification backend is unavailable or the OS does not support that operation.

`updateApply` applies the native update cached by `updateCheck`. It accepts an
optional `updateId` returned by `updateCheck`; the desktop runtime rejects a
mismatched id. `updateRestart` spawns the current executable and exits the
current runtime process.

The `files` command is typed and dispatched. It is rooted in Cefari's managed
app-data directory and rejects absolute paths and parent traversal before
filesystem access.

## Keep Types In Sync

`npm/src/app/ipc.ts` is copied from
`crates/cefari-core/bindings/ipc.ts`. When Rust IPC types change:

1. Run `cargo test -p cefari-core` to rebuild generated Rust IPC glue and check
   `crates/cefari-core/bindings/ipc.ts`.
2. Run `deno task ipc:sync` from the repository root to copy the checked-in
   core binding to `npm/src/app/ipc.ts`.
3. Update wrapper functions for new commands or result variants.
4. Update these docs when behavior or supported command status changes.
5. Run package and template checks.

Use `deno task ipc:check` in verification to confirm the core binding is fresh
and the npm copy matches it. `pnpm --dir npm build` compiles the checked-in npm
copy; it does not run Specta or copy bindings.

Generated IPC declarations should stay exact. Prose docs should explain command
intent, supported status, failure behavior, and preferred wrappers rather than
duplicating every generated type member.
