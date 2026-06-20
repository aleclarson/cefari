# Proposal: Multi-Window Native Runtime

Cefari should support multiple native windows while keeping one Vite frontend
application, one desktop runtime process, and one typed IPC contract.

## Goals

- Let frontend code create and manage secondary native windows.
- Keep the startup window addressable as the main window.
- Route window commands to either the current window or an explicit target.
- Keep window events typed and attributable to a specific window.
- Use the existing Tao and CEF runtime stack.
- Avoid committing to broad platform-specific chrome options in the first
  public API.

## Non-Goals

- Do not support arbitrary external URLs as trusted app windows.
- Do not preserve legacy single-window protocol names for compatibility alone.
- Do not add a new native webview stack.
- Do not persist arbitrary secondary window existence by default.
- Do not implement AppKit document-modal sheets in the first pass.

## Window Identity

Public window identity should use Cefari-owned string IDs.

- `main` is reserved for the startup window.
- Secondary windows may use app-supplied IDs.
- If an ID is omitted, the runtime generates IDs such as `window-1`.
- IDs must be unique among live windows.
- Tao `WindowId` values and CEF browser identifiers remain internal.

The core state shape should be:

```ts
type WindowId = string;

type WindowKind = "main" | "secondary";

type WindowState = {
  id: WindowId;
  kind: WindowKind;
  title: string;
  visible: boolean;
  focused: boolean;
  modal: boolean;
  parentId?: WindowId;
  route?: string;
};
```

## Main And Secondary Lifecycle

- The `main` window is the lifecycle anchor.
- Closing `main` closes secondary windows and exits the desktop runtime.
- Closing a secondary window destroys only that window and its CEF browser.
- Tray restore actions target `main`.
- Menu actions without a source window target the focused window when that is
  meaningful, then fall back to `main`.

Tray-enabled hide-on-close behavior is intentionally deferred. It should be a
separate app lifecycle policy rather than part of the first multi-window API.

## Frontend Routing

All native windows load trusted app frontend content.

- Development windows resolve routes against the configured Vite dev URL.
- Packaged windows load `cefari://app/index.html`.
- Packaged routes are passed as metadata, using the URL hash by default.
- The native bridge is injected only for trusted packaged and configured dev
  origins.

For example:

```text
http://localhost:5173/settings?cefariWindowId=settings
cefari://app/index.html?cefariWindowId=settings#/settings
```

History-mode packaged routing is deferred until the app-scheme resource handler
supports safe fallback to `index.html` for frontend routes.

## Window Options

The first API should expose a small cross-platform option set:

```ts
type CreateWindowOptions = {
  id?: WindowId;
  route?: string;
  title?: string;
  width?: number;
  height?: number;
  minWidth?: number;
  minHeight?: number;
  maxWidth?: number;
  maxHeight?: number;
  x?: number;
  y?: number;
  visible?: boolean;
  focused?: boolean;
  resizable?: boolean;
  decorations?: boolean;
  alwaysOnTop?: boolean;
  parentId?: WindowId;
  modal?: boolean;
  persistKey?: string;
};
```

Deferred options include transparency, fullscreen, skip-taskbar behavior,
content protection, titlebar-specific platform styling, traffic-light position,
and custom native menu assignment.

## IPC Contract

`cefari-core` should own the serializable protocol and generated TypeScript
bindings. `cefari-desktop` should own all Tao, CEF, menu, tray, event-loop, and
persistence behavior.

Window commands:

- `windowCurrent`
- `windowList`
- `windowCreate`
- `windowShow`
- `windowFocus`
- `windowClose`
- `windowSetTitle`

Commands that operate on an existing window accept an optional target:

```ts
type WindowTarget = {
  id?: WindowId;
};

type WindowSetTitleRequest = WindowTarget & {
  title: string;
};
```

If a target is omitted, the desktop runtime uses the native source window for
bridge IPC calls. Native menu, tray, and opened-URL sources fall back to `main`
unless a focused-window default is explicitly supported for that command.

Window results:

- `window` returns one `WindowState`.
- `windowList` returns all live `WindowState` values.

Window events:

- `windowCreated`
- `windowShown`
- `windowFocused`
- `windowBlurred`
- `windowCloseRequested`
- `windowClosed`
- `windowMoved`
- `windowResized`
- `windowTitleChanged`

Every window event payload includes `windowId`. Events that describe current
state include a `WindowState` snapshot.

## Frontend TypeScript API

`cefari.desktop.window` remains the current-window convenience API:

```ts
await cefari.desktop.window.current();
await cefari.desktop.window.show();
await cefari.desktop.window.focus();
await cefari.desktop.window.close();
await cefari.desktop.window.setTitle("Dashboard");
```

`cefari.desktop.windows` manages all windows:

```ts
const settings = await cefari.desktop.windows.create({
  id: "settings",
  route: "/settings",
  title: "Settings",
  width: 720,
  height: 560,
});

await cefari.desktop.windows.focus("settings");
await cefari.desktop.windows.setTitle("settings", "Preferences");
await cefari.desktop.windows.close("settings");

const all = await cefari.desktop.windows.list();
```

Event helpers should support both global and window-filtered subscriptions:

```ts
cefari.desktop.windows.onCreated((event) => {});
cefari.desktop.windows.onClosed("settings", (event) => {});
cefari.desktop.window.onFocused((event) => {});
```

## Desktop Runtime Responsibilities

`cefari-desktop` should introduce a `WindowManager` that owns live window
records.

Each record should track:

- Cefari window ID.
- Tao window.
- Tao window ID.
- CEF browser ID when available.
- kind, title, route, parent, modal, and persistence metadata.

The desktop runtime should:

- Create windows on the Tao event-loop thread.
- Attach one CEF browser to each managed window.
- Map browser-originated bridge IPC to a source Cefari window ID.
- Route resize, focus, close, reload, and DevTools actions to the correct
  browser.
- Broadcast lifecycle events to all live app windows.
- Treat browser/window close races as normal lifecycle events.

## Parent And Modal Windows

Parent and modal support should be best-effort native behavior plus explicit
Cefari state.

- macOS can use Tao parent-window support.
- Windows should use owner windows for dialog-like secondary windows.
- Linux should use transient windows where the backend supports them.
- App-level state should reject missing parents and track modal windows even
  when a platform cannot fully enforce modality.
- Closing a parent should close its secondary children.

## Persistence

Window geometry persistence should be scoped and predictable.

- Persist `main` geometry by default.
- Persist secondary geometry only when `persistKey` is supplied.
- Store size, position, maximized, and fullscreen state where available.
- Ignore corrupt persisted state with a warning.
- Do not persist secondary window existence or route by default.

## Platform Notes

- macOS activation and parent-window ordering need smoke coverage.
- Windows modal behavior should disable the owner while the modal child is
  active.
- Linux parent, position, and always-on-top behavior depends on the window
  manager and display backend.
- Wayland may not support absolute restore positions.

## Test Strategy

- Add core IPC serialization and generated binding tests.
- Add desktop dispatcher tests for source-window defaults and explicit targets.
- Add `WindowManager` unit tests for IDs, state, and lifecycle transitions.
- Add CEF/browser mapping tests for source identity and missing browser cases.
- Add TypeScript wrapper tests for current-window and all-window APIs.
- Add smoke coverage for creating, focusing, titling, and closing a secondary
  window while `main` remains alive.
- Document manual platform checks for parent/modal behavior.

## Open Questions

- Should menu commands default to the focused window for all commands or only
  window-specific commands?
- Should configured startup secondary windows be added to `cefari.config.ts`
  later?
- Should packaged history-mode frontend routing become a first-class option?
- Should app code be able to recreate a closed explicit-ID window with the same
  ID immediately after `windowClosed`?
