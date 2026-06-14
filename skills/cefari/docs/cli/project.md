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
includes `cefari.toml`, `frontend/index.html`, `daemon/main.ts`, an app README,
and a Cefari agent skill.

## `cefari dev`

Run the local development environment:

```bash
cefari dev [PATH] [--frontend-port PORT]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--frontend-port PORT`: override `[frontend].dev_port`.

Dev mode starts the frontend dev server, Deno daemon, and desktop runtime
together. When one child process exits or fails, Cefari stops the remaining
processes.

Use `--frontend-port 0` only with the built-in static server. Configured
`frontend.dev_command` values require a fixed port.

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
  metadata. Overrides `[package].version`.

`cefari package` expects `cefari build` artifacts to exist first. It writes
package metadata under `dist/package/` and invokes `cargo-packager` when that
tool is available.

## `cefari clean`

Remove generated build and dist artifacts:

```bash
cefari clean [PATH]
```

Arguments:

- `PATH`: project directory. Defaults to the current directory.

The command loads `cefari.toml` first, then removes the project's `build/` and
`dist/` directories when they exist.
