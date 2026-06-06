# Cefari Rust App Architecture Plan

Cefari separates shipped runtime code from developer tooling. The desktop app ships with only the runtime code it needs. `cefari-cli` is a public developer-facing orchestration tool, similar to Tauri's CLI, used to create, develop, build, package, sign, and prepare updates for Cefari apps.

```text
cefari/
  crates/
    cefari-core/      # reusable runtime library
    cefari-desktop/   # shipped desktop app binary / native shell
    cefari-cli/       # public developer CLI; binary name: cefari
```

Recommended package names:

```text
cefari-core
cefari-desktop
cefari-cli
```

## Core architecture rule

> `cefari-core` and `cefari-desktop` are runtime code. `cefari-cli` is developer/release orchestration tooling. The CLI is distributed to developers, but it is not bundled into the shipped desktop app or required at runtime.

## Crate roles

### `cefari-core`

Reusable runtime library used by the desktop app.

Owns:

```text
paths/config
resource resolution
logging/error types
update check/install helpers
service install/start/stop helpers
```

Does not own:

```text
Tao/CEF windowing
CLI command parsing
packaging/signing/build orchestration
frontend or daemon build steps
```

Runtime dependencies:

```toml
serde = { version = "*", features = ["derive"] }
serde_json = "*"
directories = "*"
tracing = "*"
anyhow = "*"
thiserror = "*"

service-manager = "*"
cargo-packager-updater = "*"
cargo-packager-resource-resolver = "*"
```

### `cefari-desktop`

The shipped Cefari app binary.

Owns:

```text
Tao window
CEF initialization
single-instance lock
runtime logging setup
loading the UI promptly
Rust-side update flow
daemon service management
```

Uses `cefari-core` for runtime helpers:

```toml
cefari-core = { path = "../cefari-core" }

tao = "*"
cef = "*"
raw-window-handle = "*"

single-instance = "*"
tracing-subscriber = "*"
tracing-appender = "*"
anyhow = "*"
```

Desktop dependencies:

```toml
muda = "*"       # native menus
tray-icon = "*"  # tray/menu-bar icon
open = "*"       # opening external links/files from Rust
```

### `cefari-cli`

Public developer-facing CLI. Binary name:

```bash
cefari
```

It plays the same role as Tauri's CLI: a project orchestration tool used during development and release, not a runtime component of the shipped desktop app.

Example commands:

```bash
cefari init
cefari dev
cefari build
cefari package
cefari codesign
cefari notarize
cefari make-update
cefari doctor
cefari info
```

Owns:

```text
create/scaffold Cefari projects
run dev environment
download/prepare CEF
build frontend
build Deno daemon
build Rust desktop app
package via cargo-packager
codesign/notarize via cargo-codesign
generate update artifacts
diagnostics and environment info
clean/dist tasks
```

CLI dependencies:

```toml
clap = { version = "*", features = ["derive"] }
anyhow = "*"
xshell = "*"
duct = "*"
camino = "*"
serde = { version = "*", features = ["derive"] }
toml = "*"
```

`cefari-cli` shells out to tools like `cargo-packager` and `cargo-codesign`; those tools are provided by CI or the developer environment.

## No runtime CLI surface

Cefari should not rely on CLI commands for end-user runtime behavior such as:

```text
install-service
uninstall-service
start-service
stop-service
check-update
install-update
```

Runtime update and service operations belong in `cefari-desktop` via `cefari-core`.

Development, packaging, release, diagnostics, and CI functionality belongs in `cefari-cli`.

## Tool classification

| Item                               | Belongs in                                                    |
| ---------------------------------- | ------------------------------------------------------------- |
| `cargo-packager`                   | invoked by `cefari-cli` / installed by CI or developer env     |
| `cargo-codesign`                   | invoked by `cefari-cli` / installed by CI or developer env     |
| `cargo-packager-updater`           | `cefari-core` runtime dependency                              |
| `cargo-packager-resource-resolver` | `cefari-core` runtime dependency, used by desktop app          |
| `service-manager`                  | `cefari-core` runtime dependency                              |
| `tao` / `cef`                      | `cefari-desktop` only                                         |
| `muda` / `tray-icon`               | `cefari-desktop` dependencies                                 |
| `download-cef`                     | `cefari-cli`; avoid runtime and app-build network dependency   |
| `clap`                             | `cefari-cli` only                                             |

## Acceptance criteria

- Released Cefari desktop app packages do not include or require the `cefari` CLI.
- `cefari-cli` can be published/distributed separately as the developer-facing tool.
- `cefari-desktop` contains runtime app startup, windowing, CEF, and native shell logic.
- `cefari-desktop` does not contain build, packaging, signing, or release orchestration logic.
- `cefari-core` contains reusable runtime helpers and no Tao, CEF, or CLI parsing dependencies.
- Runtime integrations for updates, resources, and service management are included in `cefari-core`.
- Project creation, dev mode, builds, packaging, signing, notarization, CEF preparation, daemon build, frontend build, diagnostics, and update artifact generation are handled by `cefari-cli` or CI.
- Dependency versions are pinned during implementation rather than left as `"*"`.

## Minimal architecture sentence

> Cefari ships a Rust desktop app backed by a reusable runtime library, while `cefari-cli` is the public developer-facing tool for creating, developing, building, packaging, signing, and preparing updates for Cefari apps.
