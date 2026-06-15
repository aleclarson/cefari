# Project Commands

Use project commands for day-to-day app development.

## `cefari init`

Create a new Cefari project:

```bash
cefari init [PATH] [--name NAME]
```

Arguments and options:

- `PATH`: directory to create. Defaults to `cefari-app`.
- `--name NAME`: application display name.

The command refuses to initialize an existing path. The generated project
includes `cefari.config.ts`, `frontend/index.html`, `daemon/main.ts`, an app README,
and a Cefari agent skill.

## `cefari dev`

Run the local development environment:

```bash
cefari dev [PATH] [--vite-port PORT]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--vite-port PORT`: override `vite.devPort`.

Dev mode starts Vite through its JavaScript API, the Deno daemon, and the
desktop runtime together. When one child process exits or fails, Cefari stops
the remaining processes. The Vite port is strict and fixed.

## `cefari build`

Build frontend, daemon, CEF resources, and desktop artifacts:

```bash
cefari build [PATH] [--release]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--release`: use Cargo's release profile when Cefari builds the desktop
  runtime from source.

Installed Cefari CLI distributions should bundle a matching `cefari-desktop`
runtime beside the `cefari` executable. Source-checkout CLI builds
`cefari-desktop` with Cargo so runtime changes are picked up during Cefari
development. Set `CEFARI_DESKTOP_RUNTIME=/path/to/cefari-desktop` to force a
specific prebuilt runtime and skip the Cargo build.

Build output is written under `build/`. See
[Build And Package](../guides/build-and-package.md) for output details.

## `cefari package`

Prepare native package assembly for a built project:

```bash
cefari package [PATH] [--release] [--release-version VERSION]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--release`: package release-profile build output when the desktop runtime
  was built from source.
- `--release-version VERSION`: package version written to native package
  metadata. Overrides `package.version`.

`cefari package` expects `cefari build` artifacts to exist first. It writes
package metadata under `dist/package/` and invokes `cargo-packager` when that
tool is available.
