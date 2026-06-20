# Project Commands

Use project commands for day-to-day app development.

## `cefari dev`

Run the local development environment:

```bash
cefari dev [PATH] [--target TARGET] [--vite-port PORT]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--target TARGET`: runtime target. Supported values are `desktop`, `ios`, and
  `android`. `desktop` is the default. Mobile targets are recognized but are not
  implemented yet.
- `--vite-port PORT`: override `vite.devPort`.

Dev mode starts Vite through its JavaScript API and the desktop runtime. When a
daemon is configured, the desktop runtime can launch it for daemon stream
connections. When one child process exits or fails, Cefari stops the remaining
processes. The Vite port is strict and fixed.

## `cefari build`

Build frontend, optional daemon, CEF resources, and desktop artifacts:

```bash
cefari build [PATH] [--release] [--target TARGET]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--release`: use Cargo's release profile when Cefari builds the desktop
  runtime from source.
- `--target TARGET`: runtime target or desktop platform/architecture target.
  Runtime targets are `desktop`, `ios`, and `android`. Desktop build targets are
  `darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `windows-x64`, and
  `windows-arm64`. Desktop is the default runtime target. Mobile targets are
  recognized but are not implemented yet.

Installed Cefari CLI distributions should bundle a matching `cefari-desktop`
runtime beside the `cefari` executable. Source-checkout CLI builds
`cefari-desktop` with Cargo so runtime changes are picked up during Cefari
development. Set `CEFARI_DESKTOP_RUNTIME=/path/to/cefari-desktop` to force a
specific prebuilt runtime and skip the Cargo build.

For non-host build targets, set the target-specific runtime variable, for
example `CEFARI_DESKTOP_RUNTIME_windows_x64=/path/to/cefari-desktop.exe`.

Build output is written under `build/`. See
[Build And Package](../guides/build-and-package.md) for output details.

## `cefari package`

Prepare native package assembly for a built project:

```bash
cefari package [PATH] [--target TARGET] [--release] [--release-version VERSION]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--target TARGET`: runtime target. Supported values are `desktop`, `ios`, and
  `android`. `desktop` is the default. Mobile targets are recognized but are not
  implemented yet.
- `--release`: package release-profile build output when the desktop runtime
  was built from source.
- `--release-version VERSION`: package version written to native package
  metadata. Overrides `package.version`.

`cefari package` expects `cefari build` artifacts to exist first. It writes
package metadata under `dist/package/` and invokes `cargo-packager` when that
tool is available.
