# Getting Started With Cefari

Cefari is a Rust desktop app runtime and a developer CLI for building apps with
a web frontend, a Deno daemon, and native desktop capabilities.

Use this page as the entry point. It branches into task-oriented guides under
`docs/guides/`.

## Prerequisites

Install the tools Cefari uses for local development and release work:

- Rust and Cargo
- Deno
- `cargo-packager` when creating native packages
- `cargo-codesign` when signing, notarizing, or generating update signatures

From a Cefari repository checkout, install the CLI globally so app projects can
run `cefari` directly:

```bash
cargo install --path crates/cefari-cli --locked
```

Released builds are distributed through npm:

```bash
npm install -g cefari
```

The npm package installs a platform-specific native `cefari` binary and the
matching `cefari-desktop` runtime for supported macOS arm64, macOS x64, Linux
x64, and Windows x64 hosts.

Check the local environment:

```bash
cefari doctor
```

## Create An App

Create a minimal project:

```bash
cefari init my-cefari-app --name "My Cefari App"
```

The generated `cefari.toml` includes:

- `[app].project_name`, a lowercase machine name used for executable output
  names
- `[app].name` and `[app].identifier`
- `[frontend].dist` and `[frontend].dev_port`
- `[daemon].entry`
- `[package].product_name`

For full scaffolding guidance, see [Scaffold An App](guides/scaffolding.md).

## Run In Development

Start the local development environment:

```bash
cefari dev my-cefari-app
```

Cefari starts the frontend, Deno daemon, and Rust desktop app together. Use
`--frontend-port PORT` when a project needs a specific frontend port.

For Vite and custom frontend commands, see
[Develop Locally](guides/development.md).

For frontend TypeScript code that calls native Cefari actions, see
[TypeScript App Guide](typescript/index.md).

## Build And Package

Build local app artifacts:

```bash
cefari build my-cefari-app
```

Prepare native package assembly:

```bash
cefari package my-cefari-app
```

For release-profile builds, package prerequisites, and generated outputs, see
[Build And Package](guides/build-and-package.md).

## Deploy

The repository includes a composite GitHub Action at
`.github/actions/cefari-release`. The Vite React template includes production
and prerelease workflow stubs that call that action.

For automated release setup, signing, update metadata, and prerelease dry runs,
see [Automated Deployment](guides/deployment.md).

## App Developer References

- [CLI Reference](cli/index.md): command syntax for development, release, and
  diagnostics.
- [`cefari.toml` Reference](config/index.md): project manifest fields and
  validation rules.
- [Cefari CSS Contract](css-contract.md): opt-in drag-region utility classes.
- [TypeScript App Guide](typescript/index.md): task-oriented `@cefari/app`
  usage from frontend code.
- [Notification Behavior](notifications.md): runtime-owned OS notification
  boundary.
