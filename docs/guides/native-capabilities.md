# Native Capabilities

Cefari routes native desktop behavior through Rust runtime code, not through ad
hoc frontend calls.

## Rust-Owned Areas

The desktop runtime owns:

- window lifecycle actions
- native menu actions
- tray/menu-bar actions
- opening validated external URLs
- opening the runtime log location
- update checks and installs
- daemon service status and lifecycle helpers
- OS notification setup

## TypeScript APIs

Frontend code should use `cefari/app` for native desktop capabilities instead
of inventing stringly typed native commands. See the
[TypeScript App Guide](../typescript/index.md).

## Adding IPC Capability Work

Keep native feature work vertical. A feature agent should usually touch one
capability module in each layer, plus its TypeScript wrapper and tests:

- `crates/cefari-core/src/ipc/<capability>.rs` for command, result, event, and
  payload types owned by the capability.
- `crates/cefari-core/src/ipc/capabilities/<capability>.rs` for generated
  top-level IPC glue metadata.
- `crates/cefari-desktop/src/desktop_ipc/<capability>.rs` for typed dispatch
  from IPC commands to native runtime behavior.
- The concrete desktop runtime module that owns the native behavior, such as
  `desktop_files`, `desktop_notifications`, `window`, or update/service code.
- `crates/cefari-desktop/src/shell_context.rs` only when the capability needs a
  new runtime adapter method.
- `crates/cefari-desktop/src/desktop_bridge.rs` when bridge tests or fake
  dispatch behavior need the new command.
- `npm/src/app/<capability>.ts` and `npm/tests/app/cefari_app_test.ts` for the
  public TypeScript wrapper.
- User-facing docs under `docs/typescript/` and `docs/spec.md` when the change
  adds supported product behavior.

Avoid adding unrelated commands to `desktop_ipc/mod.rs` or hand-editing the
top-level IPC enums. The shared `CefariIpcCommand`, `CefariIpcResult`,
`CefariIpcEvent`, and `ipc_types()` glue is generated from capability metadata
during the `cefari-core` build.

Capability metadata files are Rust-shaped generator inputs. Each file declares
the capability name, stable ordering, and the top-level command, result, and
event variants contributed by that capability:

```rust
capability! {
    name: files,
    order: 100,
    commands: [
        Files(FilesCommand),
    ],
    results: [
        File(FileResult),
    ],
    events: [
    ],
}
```

Use a nested command enum, such as `FilesCommand`, when a capability owns
multiple operations. Use a direct top-level command variant only for small
single-operation capabilities whose existing wire shape should stay flat.

Run these checks after changing IPC contracts:

```bash
cargo fmt --all --check
cargo test -p cefari-core
cargo test -p cefari-desktop desktop_ipc
pnpm --dir npm build
pnpm --dir npm check
deno test tests/app
```

Run `deno test tests/app` from the `npm/` directory. `cargo test -p cefari-core`
rebuilds the generated Rust IPC glue, rejects duplicate capability names or
top-level wire tags, and verifies `crates/cefari-core/bindings/ipc.ts` is
current. `pnpm --dir npm build` refreshes `npm/src/app/ipc.ts` from the checked
in core binding before TypeScript checks run.

## Notifications

Notification delivery is owned by the desktop runtime. Startup prepares
notification support but does not send notifications or request permission. See
[Notification Behavior](../notifications.md).

The notification capability includes permission checks, permission prompts where
the OS supports them, rich delivery fields, category/action registration,
inline reply payloads where supported, response events, active notification
listing, delivered notification removal, and packaged activation metadata.
Support for each field is platform-dependent; the platform matrix in
[Notification Behavior](../notifications.md) is the source of truth.

## Custom Titlebars

Use Cefari's CSS contract for drag regions. See
[Cefari CSS Contract](../css-contract.md).
