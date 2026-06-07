# Dependency Notes

Dependency placement follows the runtime/developer tooling boundary in [architecture.md](architecture.md).

## Runtime Library

`cefari-core` may depend on runtime-safe libraries for configuration, paths, resources, logging types, updates, and service management.

Allowed current runtime dependencies:

- `serde`, `serde_json`
- `directories`
- `tracing`
- `anyhow`, `thiserror`
- `cargo-packager-resource-resolver`
- `cargo-packager-updater`
- `service-manager`

`cefari-core` must not depend on CLI orchestration crates such as `clap`, `xshell`, `duct`, `camino`, or `toml`. It must not depend on desktop windowing or browser crates such as `tao` or `cef`.

## Desktop App

`cefari-desktop` owns native shell dependencies:

- `tao`
- optional `cef`
- `raw-window-handle`
- `single-instance`
- `tracing`, `tracing-subscriber`, `tracing-appender`
- `muda`
- `tray-icon`
- `open`

The `cef` dependency is optional until CEF initialization is implemented and verified. Making it required is tracked in `todo.md`.

## CLI

`cefari-cli` owns developer orchestration dependencies:

- `clap`
- `xshell`
- `duct`
- `camino`
- `serde`, `toml`
- `anyhow`, `thiserror`

`cefari-cli` must not pull in `tao` or `cef`.

## Audit Commands

```bash
cargo tree -p cefari-core | rg 'clap|xshell|duct|camino|toml|tao|cef ' || true
cargo tree -p cefari-cli | rg 'tao|cef ' || true
cargo tree -p cefari-desktop --depth 1
```

