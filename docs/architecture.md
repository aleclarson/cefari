# Architecture Boundary

Cefari keeps shipped runtime code separate from developer and release tooling.

## Crates

- `cefari-core`: reusable runtime library for config, paths, resources, logging inputs, update helpers, service helpers, and shared IPC contract types.
- `cefari-desktop`: shipped desktop runtime. It owns windowing, CEF startup, native menus, tray integration, notifications, runtime logging setup, and native action dispatch.
- `npm`: developer tool distributed separately from Cefari apps. It owns scaffolding, dev orchestration, frontend and daemon builds, desktop builds, package assembly, signing, notarization, update artifact generation, diagnostics, and cleanup.

## Boundary Rules

- Runtime crates must not depend on CLI-only orchestration.
- `cefari-core` must not own windowing, CEF startup, native menu/tray setup, or CLI command parsing.
- `npm` should not pull desktop windowing or browser runtime responsibilities into developer tooling.
- Shared behavior that both Rust runtime and frontend code need should be expressed as a stable contract, such as the Specta-generated IPC types.

## Related Guides

- [Scaffold An App](guides/scaffolding.md)
- [Develop Locally](guides/development.md)
- [Build And Package](guides/build-and-package.md)
- [Native Capabilities](guides/native-capabilities.md)
