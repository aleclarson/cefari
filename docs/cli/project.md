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

Dev mode starts the frontend dev server, Deno daemon, and desktop app together.
When one child process exits or fails, Cefari stops the remaining processes.

Use `--frontend-port 0` only with the built-in static server. Configured
`frontend.dev_command` values require a fixed port.

## `cefari build`

Build frontend, daemon, CEF resources, and desktop artifacts:

```bash
cefari build [PATH] [--release]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--release`: build the desktop runtime with Cargo's release profile.

Build output is written under `build/`. See
[Build And Package](../guides/build-and-package.md) for output details.

## `cefari package`

Prepare native package assembly for a built project:

```bash
cefari package [PATH] [--release]
```

Arguments and options:

- `PATH`: project directory. Defaults to the current directory.
- `--release`: package the desktop runtime from Cargo's release profile output.

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
