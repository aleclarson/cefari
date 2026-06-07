# Cefari IPC Protocol

Cefari IPC payloads are defined in Rust in `cefari-core::ipc` and exported to TypeScript in `crates/cefari-core/bindings/ipc.ts` with Specta.

The initial command surface reserves typed payloads for:

- app quit
- window show, focus, close, and set-title
- open logs
- reload UI
- validated external URL opening
- update state and update check
- daemon service status
- tray restore-window
- notification permission/send commands

Notification commands are protocol-reserved until the dispatcher exposes notification behavior. They may return an `unsupported` error while the transport and dispatcher are being wired.

All responses use a request `id` and a typed `outcome`. Error responses use explicit `invalidCommand`, `denied`, `unknownCommand`, and `unsupported` variants.

## Dispatcher Boundary

`cefari-desktop` routes native menu, tray, window, update, service, logs, and external URL actions through the typed dispatcher. CEF transport should call that same dispatcher instead of adding a separate native-action path.

## Bridge Policy

The desktop bridge installs `window.cefari` only for trusted packaged app origins and allowed localhost development origins. Requests from other origins receive a typed `denied` response. Unknown command tags receive `unknownCommand`, malformed requests receive `invalidCommand`, and reserved but unavailable commands receive `unsupported`.

The trusted main-frame bridge bootstrap also installs Cefari's default CSS contract for drag regions. See [css-contract.md](css-contract.md).
