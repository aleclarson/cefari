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

Reserved commands can be present in generated types before the desktop
dispatcher supports them. At the moment:

- `reloadUi` is typed and wrapped, but the dispatcher returns `unsupported`.
- `notification` commands expose the full typed permission, capability,
  category, delivery, management, and response contract, but the current
  desktop dispatcher returns `unsupported` until native dispatch is wired.

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

1. Regenerate `crates/cefari-core/bindings/ipc.ts`.
2. Copy it to `npm/src/app/ipc.ts`.
3. Update wrapper functions for new commands or result variants.
4. Update these docs when behavior or supported command status changes.
5. Run package and template checks.

Generated IPC declarations should stay exact. Prose docs should explain command
intent, supported status, failure behavior, and preferred wrappers rather than
duplicating every generated type member.
