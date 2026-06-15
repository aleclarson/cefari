# Build And Package

Build a Cefari project:

```bash
cefari build PATH
```

Package a built project:

```bash
cefari package PATH
```

If `PATH` is omitted, Cefari uses the current directory.

## Build Outputs

`cefari build` writes generated artifacts under `build/`:

- `build/frontend/`
- `build/daemon/main.ts`
- `build/daemon/<projectName>-daemon`
- `build/desktop/<projectName>`
- `build/cef/resources/`
- `build/cef/resources/archive.json`
- `build/cef/manifest.json`

On Windows, executable names use `.exe`.

For release-profile desktop builds:

```bash
cefari build PATH --release
```

## Vite Builds

`cefari build` calls Vite's `build` API directly and forces the output directory
to `build/frontend/`.

## Daemon Builds

The daemon entry is configured by `daemon.entry`. Cefari keeps a source copy
at `build/daemon/main.ts` and compiles the daemon into the project-named daemon
executable.

## Desktop Runtime

`cefari build` copies a matching `cefari-desktop` runtime into
`build/desktop/<projectName>`. Installed Cefari CLI distributions should bundle
that runtime beside the `cefari` executable so app developers do not need to
compile the Rust desktop dependency tree.

When Cefari is running from a source checkout, the CLI still builds
`cefari-desktop` with Cargo so local runtime changes are picked up. Set
`CEFARI_DESKTOP_RUNTIME=/path/to/cefari-desktop` to force a specific prebuilt
runtime and skip the Cargo build.

## CEF Resources

`cefari build` prepares CEF resources as part of the build. The package step
expects those resources and archive metadata to exist under `build/cef/`.
Native packages include the prepared CEF resource directory as the package
resource target `cef`; at runtime, Cefari uses that directory for CEF resources,
locales when present, and platform framework files when present.
Packaging verifies that the desktop binary used as the CEF subprocess is present,
that `archive.json` exists, that the CEF resource directory contains runtime
payload files, and that `cef/locales/` contains at least one locale file.

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

For release-profile packaging:

```bash
cefari package PATH --release --release-version 1.2.3
```

Without `--release-version`, package metadata uses `package.version` from
`cefari.config.ts`.

For command syntax, see [Project Commands](../cli/project.md). For package
config fields, see [`cefari.config.ts` Reference](../config/index.md).
