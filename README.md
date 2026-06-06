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

> `cefari-core` and `cefari-desktop` are runtime code. `cefari-cli` is developer/release orchestration tooling. The CLI may be distributed to developers, but it is not bundled into the shipped desktop app or required at runtime.

## Crate roles

### `cefari-core`

Reusable runtime library used by the desktop app.

Owns:

```text
paths/config
resource resolution
logging/error types
update check/install helpers, if runtime updates are enabled
service install/start/stop helpers, if Cefari manages a daemon service
```

Does not own:

```text
Tao/CEF windowing
CLI command parsing
packaging/signing/build orchestration
frontend or daemon build steps
```

Suggested feature gates:

```toml
[features]
services = ["dep:service-manager"]
updates = ["dep:cargo-packager-updater"]
resources = ["dep:cargo-packager-resource-resolver"]
```

Typical dependencies:

```toml
serde = { version = "*", features = ["derive"] }
serde_json = "*"
directories = "*"
tracing = "*"
anyhow = "*"
thiserror = "*"

service-manager = { version = "*", optional = true }
cargo-packager-updater = { version = "*", optional = true }
cargo-packager-resource-resolver = { version = "*", optional = true }
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
optional Rust-side update flow
optional daemon service management
```

Uses `cefari-core` for runtime helpers:

```toml
cefari-core = { path = "../cefari-core", features = ["updates", "resources", "services"] }

tao = "*"
cef = "*"
raw-window-handle = "*"

single-instance = "*"
tracing-subscriber = "*"
tracing-appender = "*"
anyhow = "*"
```

Optional desktop-only dependencies:

```toml
muda = "*"       # native menus only
tray-icon = "*"  # tray/menu-bar icon only
open = "*"       # opening external links/files from Rust only
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
clean/dist tasks if needed
```

Typical dependencies:

```toml
clap = { version = "*", features = ["derive"] }
anyhow = "*"
xshell = "*"
duct = "*"
camino = "*"
serde = { version = "*", features = ["derive"] }
toml = "*"
```

`cefari-cli` may shell out to tools like `cargo-packager` and `cargo-codesign`; those tools can be installed by CI, the developer environment, or managed by the CLI if Cefari chooses to support that.

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

If the functionality is needed by the shipped app at runtime, it belongs in `cefari-desktop` via `cefari-core`.

If the functionality is needed for development, packaging, release, diagnostics, or CI, it belongs in `cefari-cli`.

## Tool classification

| Item                               | Belongs in                                                    |
| ---------------------------------- | ------------------------------------------------------------- |
| `cargo-packager`                   | invoked by `cefari-cli` / installed by CI or developer env     |
| `cargo-codesign`                   | invoked by `cefari-cli` / installed by CI or developer env     |
| `cargo-packager-updater`           | optional `cefari-core` runtime dependency                     |
| `cargo-packager-resource-resolver` | optional `cefari-core` runtime dependency, used by desktop app |
| `service-manager`                  | optional `cefari-core` runtime dependency if Cefari manages daemon |
| `tao` / `cef`                      | `cefari-desktop` only                                         |
| `muda` / `tray-icon`               | optional `cefari-desktop` dependencies                        |
| `download-cef`                     | `cefari-cli`; avoid runtime and app-build network dependency   |
| `clap`                             | `cefari-cli` only                                             |

## Acceptance criteria

- Released Cefari desktop app packages do not include or require the `cefari` CLI.
- `cefari-cli` can be published/distributed separately as the developer-facing tool.
- `cefari-desktop` contains runtime app startup, windowing, CEF, and native shell logic.
- `cefari-desktop` does not contain build, packaging, signing, or release orchestration logic.
- `cefari-core` contains reusable runtime helpers and no Tao, CEF, or CLI parsing dependencies.
- Runtime integrations in `cefari-core` are feature-gated where appropriate.
- Project creation, dev mode, builds, packaging, signing, notarization, CEF preparation, daemon build, frontend build, diagnostics, and update artifact generation are handled by `cefari-cli` or CI.
- Dependency versions are pinned during implementation rather than left as `"*"`.

## Minimal architecture sentence

> Cefari ships a Rust desktop app backed by a reusable runtime library, while `cefari-cli` is the public developer-facing tool for creating, developing, building, packaging, signing, and preparing updates for Cefari apps.
