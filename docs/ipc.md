# Cefari IPC Protocol

Cefari IPC payloads are defined in Rust in `cefari-core::ipc` and exported to TypeScript in `crates/cefari-core/bindings/ipc.ts` with Specta.

The initial command surface reserves typed payloads for:

- app quit
- current-window lookup and live window listing
- secondary window creation
- target-aware window show, focus, close, and set-title
- open logs
- reload UI
- validated external URL opening
- update state, update check, update apply, and update restart
- daemon service status
- tray restore-window
- notification permission, capability, category, delivery, management, and
  response commands
- sandboxed app-data file commands

Update apply commands install the native update cached by the most recent
successful update check. Frontend code does not pass update URLs, signatures, or
installer paths through IPC. Notification commands expose the full typed
permission, capability, category, delivery, management, and response-event
contract through the desktop dispatcher. Individual notification operations may
still return `unsupported` when the desktop backend is unavailable or an OS
operation is not supported.

File commands are rooted in Cefari's managed app-data directory. They support
text and base64 reads/writes, directory listing, directory creation, removal,
rename, copy, stat, and access checks. The desktop dispatcher rejects absolute
paths and parent traversal before invoking the `cap-std` directory capability.

Window creation payloads support app-defined IDs, frontend routes, initial
geometry, parent/modal options, and opt-in secondary geometry persistence through
`persistKey`. Cefari emits typed lifecycle events for window create, show,
focus, blur, close-request, close, move, resize, and title changes. Event
payloads include the Cefari window ID so frontend code can route events to the
right view.

All responses use a request `id` and a typed `outcome`. Error responses use explicit `invalidCommand`, `denied`, `unknownCommand`, and `unsupported` variants.

## Dispatcher Boundary

`cefari-desktop` routes native menu, tray, window, update, service, logs, and external URL actions through the typed dispatcher. CEF transport should call that same dispatcher instead of adding a separate native-action path.

## Bridge Policy

The desktop bridge installs `window.cefari` only for trusted packaged app origins and allowed localhost development origins. Requests from other origins receive a typed `denied` response. Unknown command tags receive `unknownCommand`, malformed requests receive `invalidCommand`, and reserved but unavailable commands receive `unsupported`.

The trusted main-frame bridge bootstrap also installs Cefari's default CSS contract for drag regions. See [Cefari CSS Contract](css-contract.md).

For task-oriented guidance, see [Native Capabilities](guides/native-capabilities.md)
and the [`cefari/app` TypeScript guide](typescript/index.md).
