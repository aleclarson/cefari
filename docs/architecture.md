# Cefari Architecture Boundary

Cefari is split into runtime code and developer tooling.

- `cefari-core` is the reusable runtime library for paths, config, resource lookup, logging inputs, update helpers, and service helpers.
- `cefari-desktop` is the shipped desktop app binary. It owns the Tao window, CEF initialization, single-instance locking, runtime logging setup, native menus, tray integration, OS notification wiring, update flow wiring, and daemon service wiring.
- `cefari-cli` is the separately distributed developer tool. It owns project creation, development orchestration, frontend and daemon builds, desktop builds, packaging, signing, notarization, update artifact generation, and diagnostics.

Runtime crates must not depend on CLI-only orchestration code. The CLI must not introduce Tao or CEF into runtime libraries.
