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
- `build/daemon/<project_name>-daemon`
- `build/desktop/<project_name>`
- `build/cef/resources/`
- `build/cef/resources/archive.json`
- `build/cef/manifest.json`

On Windows, executable names use `.exe`.

For release-profile desktop builds:

```bash
cefari build PATH --release
```

## Frontend Builds

When `[frontend].build_command` is configured, Cefari runs it before copying
`[frontend].dist` into `build/frontend/`.

Without a build command, Cefari preserves the minimal scaffold behavior by
copying `frontend/index.html`.

## Daemon Builds

The daemon entry is configured by `[daemon].entry`. Cefari keeps a source copy
at `build/daemon/main.ts` and compiles the daemon into the project-named daemon
executable.

## CEF Resources

`cefari build` prepares CEF resources as part of the build. The package step
expects those resources and archive metadata to exist under `build/cef/`.

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
cefari package PATH --release
```

## Clean Generated Artifacts

Remove generated build and package outputs:

```bash
cefari clean PATH
```
