# Architecture Boundary

Cefari keeps shipped runtime code separate from developer and release tooling.

## Crates

- `cefari-core`: reusable runtime library for config, paths, resources, logging inputs, update helpers, service helpers, and shared IPC contract types.
- `cefari-desktop`: shipped desktop runtime. It owns windowing, CEF startup, native menus, tray integration, notifications, runtime logging setup, and native action dispatch.
- `npm`: developer tool distributed separately from Cefari apps. It owns scaffolding, dev orchestration, frontend and optional daemon builds, desktop builds, package assembly, signing, notarization, update artifact generation, diagnostics, and cleanup.

## Boundary Rules

- Runtime crates must not depend on CLI-only orchestration.
- `cefari-core` must not own windowing, CEF startup, native menu/tray setup, or CLI command parsing.
- `npm` should not pull desktop windowing or browser runtime responsibilities into developer tooling.
- Shared behavior that both Rust runtime and frontend code need should be expressed as a stable contract, such as the Specta-generated IPC types.

## Rust IPC Modularity

Cefari's Rust IPC surface should stay statically typed while avoiding broad
shared edit points for unrelated native features.

The wire boundary remains a single request and response envelope:

- `CefariIpcRequest`
- `CefariIpcResponse`
- `CefariIpcCommand`
- `CefariIpcResult`
- `CefariIpcEvent`

Inside that envelope, commands, results, events, and payload types should be
owned by capability modules. The intended core layout is:

```text
crates/cefari-core/src/ipc/
  mod.rs
  app.rs
  windows.rs
  shell.rs
  updates.rs
  service.rs
  tray.rs
  downloads.rs
  notifications.rs
  dialogs.rs
  files.rs
```

The desktop runtime should mirror those capability boundaries for dispatch:

```text
crates/cefari-desktop/src/desktop_ipc/
  mod.rs
  app.rs
  windows.rs
  shell.rs
  updates.rs
  service.rs
  tray.rs
  downloads.rs
  notifications.rs
  dialogs.rs
  files.rs
```

Each desktop capability owns its dispatch function and the smallest context
trait needed to execute that capability. `DesktopShellContext` remains the
runtime adapter, but it should implement small capability traits instead of one
large native shell trait.

Top-level IPC assembly glue may be generated when that removes shared manual
edits. Generated code should be limited to boring glue, such as top-level enum
assembly and Specta registration. Capability payload types, validation,
dispatch bodies, native behavior, and tests should remain hand-written.

Capability metadata for generated glue should live next to the Rust capability
module, or follow a strict Rust module convention. New capabilities should not
need to edit a central registry file. The generator should discover capability
metadata deterministically and reject duplicate names or duplicate wire tags.

Dynamic command registration is not the default IPC architecture. It weakens
the typed Rust and TypeScript contract, makes command coverage harder to audit,
and moves errors from compile time into runtime lookup. Use dynamic registration
only for internal extension points where there is no stable frontend contract.

The main merge hotspots this architecture avoids are the top-level IPC enums,
the desktop dispatcher match, the broad shell context trait, and generated
TypeScript binding churn. Generated artifacts may still change for parallel IPC
work, but they should be deterministic outputs of capability-owned source.

## Related Guides

- [Scaffold An App](guides/scaffolding.md)
- [Develop Locally](guides/development.md)
- [Build And Package](guides/build-and-package.md)
- [Native Capabilities](guides/native-capabilities.md)
- [Mobile Platform Readiness](proposals/mobile-platform-readiness.md)
