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
[TypeScript App Guide](guides/typescript.md).

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

## Built-In App Contracts

- [Cefari CSS Contract](css-contract.md): opt-in drag-region utility classes.
- [Cefari IPC Protocol](ipc.md): typed Rust-to-CEF native action protocol.
- [TypeScript App Guide](guides/typescript.md): task-oriented `@cefari/app`
  usage from frontend code.
- [Desktop Notifications](notifications.md): runtime-owned OS notification
  boundary.
- [Architecture Boundary](architecture.md): crate responsibilities and
  dependency boundaries.

## Claim Verification

The docs were checked against the current source layout, CLI definitions,
template manifests, GitHub Action metadata, and workflow files. See
[Documentation Verification](verification.md) for the evidence used.
