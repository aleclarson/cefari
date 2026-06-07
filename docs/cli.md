# Cefari CLI

`cefari-cli` builds the `cefari` developer tool. It is distributed separately from the shipped desktop app.

## Commands

### `cefari init [PATH] [--name NAME]`

Creates a Cefari project at `PATH`. If `PATH` is omitted, the CLI creates `./cefari-app`.

Generated files:

- `cefari.toml`
- `frontend/index.html`
- `daemon/main.ts`
- `README.md`

The generated `cefari.toml` currently contains:

- `[app]` name and identifier
- `[frontend]` dist path
- `[daemon]` entry path
- `[package]` product name

The command refuses to overwrite an existing path.

### `cefari doctor`

Prints local tool availability for:

- `cargo`
- `deno`
- `cargo-packager`
- `cargo-codesign`

Missing tools are reported as `missing`; the command does not install them.

### `cefari info`

Prints the CLI version, target OS, target architecture, and project details when run from a directory containing `cefari.toml`.

### `cefari build [PATH]`

Builds the Cefari project at `PATH`. If `PATH` is omitted, the CLI builds the current directory.

Current build behavior:

- copies `frontend/index.html` into `build/frontend/index.html`
- copies `frontend/index.html` into the configured frontend dist path
- copies the configured daemon entry into `build/daemon/main.ts`
- compiles the configured daemon entry with `deno compile` into `build/daemon/cefari-daemon` or `build/daemon/cefari-daemon.exe`
- downloads, verifies, and extracts minimal CEF resources into `build/cef/resources/`
- caches CEF downloads and extracted intermediates under `build/cef-cache/`
- writes CEF metadata to `build/cef/manifest.json` and `build/cef/resources/archive.json`
- runs `cargo build -p cefari-desktop` through the Cefari workspace manifest

Package metadata records the compiled daemon executable and verified CEF archive metadata explicitly.

### `cefari dev [PATH] [--frontend-port PORT]`

Runs the local Cefari development environment for the project at `PATH`. If `PATH` is omitted, the CLI uses the current directory.

Current dev behavior:

- starts a built-in static frontend dev server for `frontend/index.html`
- runs `deno run --watch --allow-read --allow-net` for the configured daemon entry
- runs `cargo run -p cefari-desktop` through the Cefari workspace manifest
- stops the remaining processes when one child process exits or fails

Use `--frontend-port 0` to bind the frontend server to any available local port.

### `cefari package [PATH]`

Prepares package assembly metadata for the Cefari project at `PATH`. If `PATH` is omitted, the CLI uses the current directory.

The command requires `cefari build` artifacts to exist first.

Current output:

- `dist/package/cargo-packager.toml`
- `dist/package/manifest.json`

If `cargo-packager` is available on `PATH`, `cefari package` invokes it with the generated config and writes native package output under `dist/package/output`. If the tool is missing, the command leaves the assembly metadata in place and reports that the native package step was skipped.

### `cefari codesign ARTIFACT [--platform PLATFORM] [--config PATH]`

Invokes `cargo-codesign` for a packaged artifact. `PLATFORM` defaults to the current host platform and can be `macos`, `windows`, or `linux`.

Current behavior:

- macOS: runs `cargo-codesign codesign macos --app ARTIFACT --skip-notarize` for `.app` bundles or `--dmg ARTIFACT --skip-notarize` for `.dmg` files
- Windows: runs `cargo-codesign codesign windows`
- Linux: runs `cargo-codesign codesign linux --archive ARTIFACT`

### `cefari notarize ARTIFACT [--config PATH]`

Invokes the macOS notarization flow through `cargo-codesign`. The artifact must be a `.app` bundle or `.dmg` file.

### `cefari make-update ARCHIVE --url URL --version VERSION`

Signs a release archive through `cargo-codesign codesign update` and writes update metadata compatible with `cargo-packager-updater`.

Useful options:

- `--target TARGET`: updater platform key, defaulting to the current OS and architecture
- `--format FORMAT`: updater package format, one of `app`, `appimage`, `nsis`, or `wix`; defaults from `--target`
- `--key-env NAME`: environment variable read by `cargo-codesign` for the update signing key, defaulting to `UPDATE_SIGNING_KEY`
- `--output-dir PATH`: output directory for the signature and `update.json`, defaulting to `dist/update`

Current output:

- `<ARCHIVE file name>.sig`
- `update.json`

### `cefari clean [PATH]`

Removes generated build artifacts for the Cefari project at `PATH`. If `PATH` is omitted, the CLI uses the current directory.

Current cleanup behavior:

- removes `build/`
- removes `dist/`

## Troubleshooting `cefari doctor`

- `cargo: missing` means Rust or Cargo is not on `PATH`.
- `deno: missing` means daemon development/build commands will not be able to run Deno until it is installed and on `PATH`.
- `cargo-packager: missing` means `cefari package` will not be able to package apps until the tool is installed or provided by CI.
- `cargo-codesign: missing` means signing commands will not be able to run until the tool is installed or provided by CI.
