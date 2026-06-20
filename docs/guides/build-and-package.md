# Build And Package

Build a Cefari project:

```bash
cefari build PATH
cefari build PATH --target windows-x64
```

Package a built project:

```bash
cefari package PATH
```

If `PATH` is omitted, Cefari uses the current directory.

If `--target` is omitted, Cefari builds for the host platform and architecture.
Supported build targets are `darwin-arm64`, `darwin-x64`, `linux-x64`,
`linux-arm64`, `windows-x64`, and `windows-arm64`.

## Build Outputs

`cefari build` writes generated artifacts under `build/`:

- `build/frontend/`
- `build/config/cefari.json`
- `build/daemon/main.ts`, when `daemon.entry` is configured
- `build/daemon/<projectName>-daemon`, when `daemon.entry` is configured
- `build/desktop/<projectName>`
- `build/workers/<workerId>/<workerExecutableName>`
- `build/cef/resources/`
- `build/cef/resources/archive.json`
- `build/cef/manifest.json`

Windows targets use `.exe` executable names. Non-Windows targets do not.

For release-profile desktop builds:

```bash
cefari build PATH --release
```

## Vite Builds

`cefari build` calls Vite's `build` API directly and forces the output directory
to `build/frontend/`.

## Daemon Builds

The daemon entry is configured by `daemon.entry`. When configured, Cefari keeps
a source copy at `build/daemon/main.ts` and compiles the daemon into the
project-named daemon executable. When omitted, Cefari skips daemon build and
package artifacts.

## Desktop Runtime

`cefari build` copies a matching `cefari-desktop` runtime into
`build/desktop/<projectName>`. Installed Cefari CLI distributions should bundle
that runtime beside the `cefari` executable so app developers do not need to
compile the Rust desktop dependency tree.

When Cefari is running from a source checkout, the CLI still builds
`cefari-desktop` with Cargo so local runtime changes are picked up. Set
`CEFARI_DESKTOP_RUNTIME=/path/to/cefari-desktop` to force a specific prebuilt
runtime and skip the Cargo build.

Cross-target builds require a desktop runtime for the requested target. Set a
target-specific runtime environment variable such as
`CEFARI_DESKTOP_RUNTIME_linux_x64=/path/to/cefari-desktop` when building for a
non-host target.

## Runtime Config

`cefari build` writes `build/config/cefari.json` for the desktop runtime. The
file contains the app identity, app version, configured deep-link schemes, and
whether a daemon executable is available after packaging. It also contains
metadata with packaged worker executable paths.

## Worker Builds

Configured workers are authored as Deno source scripts. `cefari build` compiles
each configured worker with `deno compile` and writes an executable under
`build/workers/<workerId>/`.

When a build target is selected, Cefari passes the corresponding Deno
`--target` triple to worker and daemon compilation.

Packaged apps launch compiled worker executables directly. They do not require a
system `deno` executable for workers at runtime.

Cefari passes per-worker permissions from `cefari.config.ts` to
`deno compile`; packaged worker permissions are baked into the executable.
Changing worker permissions requires rebuilding.

## CEF Resources

`cefari build` prepares CEF resources as part of the build. The package step
expects those resources and archive metadata to exist under `build/cef/`.
Native packages include the prepared CEF resource directory as the package
resource target `cef`; at runtime, Cefari uses that directory for CEF resources,
locales when present, and platform framework files when present.
Packaging verifies that the desktop binary used as the CEF subprocess is present,
that `build/config/cefari.json` exists, that `archive.json` exists, that the CEF
resource directory contains runtime payload files, and that `cef/locales/`
contains at least one locale file. It also includes `build/workers/` as the
package resource target `workers` and validates configured worker executables.

On macOS, packages that include `Chromium Embedded Framework.framework` must be
signed and notarized with that framework payload intact. Run
`cefari package sign` and `cefari package notarize` for release artifacts that
target macOS.

For deterministic tests or CI fixtures, `CEFARI_CEF_RESOURCES_DIR` may point at
a pre-populated resources directory that contains `archive.json`.

## Package Outputs

`cefari package` requires build artifacts to exist first. It writes package
assembly metadata under `dist/package/`:

- `cargo-packager.toml`
- `manifest.json`

When `cargo-packager` is available, Cefari invokes it and writes native package
output under `dist/package/output/`. When it is not available, Cefari leaves the
metadata in place and reports that native package generation was skipped.

Configured deep-link schemes are written to Cargo Packager metadata during this
step. Packaged apps use that metadata for OS protocol registration.

For release-profile packaging:

```bash
cefari package PATH --release --release-version 1.2.3
```

Without `--release-version`, package metadata uses `package.version` from
`cefari.config.ts`.

For command syntax, see [Project Commands](../cli/project.md). For package
config fields, see [`cefari.config.ts` Reference](../config/index.md).
