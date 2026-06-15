# Getting Started With Cefari

Cefari builds Vite apps with a native desktop shell, a Deno daemon, packaging,
updater support, and native desktop capabilities.

Use this page as the entry point. It branches into task-oriented guides under
`docs/guides/`.

## Prerequisites

Install the tools Cefari uses for local development and release work:

- Node.js
- pnpm
- Rust and Cargo
- Deno 2.8+
- `cargo-packager` when creating native packages
- `cargo-codesign` when signing, notarizing, or generating update signatures

Install the developer-facing CLI from the npm registry:

```bash
pnpm add -g cefari
```

The npm package installs the TypeScript/Node `cefari` command.

## Create An App

Create a minimal project:

```bash
cefari init my-cefari-app --name "My Cefari App"
```

The generated `cefari.config.ts` includes:

- `app.projectName`, a lowercase machine name used for executable output
  names
- `app.name` and `app.identifier`
- `vite.root`, `vite.configFile`, and `vite.devPort`
- `daemon.entry`
- `package.productName`
- `package.version`

For full scaffolding guidance, see [Scaffold An App](guides/scaffolding.md).

## Run In Development

Start the local development environment:

```bash
cefari dev my-cefari-app
```

Cefari starts Vite, the Deno daemon, and the desktop app together. Use
`--vite-port PORT` when a project needs a specific Vite port.

For Vite development details, see [Develop Locally](guides/development.md).

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

- [CLI Reference](cli/index.md): command syntax for development, package, and
  release workflows.
- [`cefari.config.ts` Reference](config/index.md): project config fields and
  validation rules.
- [Cefari CSS Contract](css-contract.md): opt-in drag-region utility classes.
- [TypeScript App Guide](typescript/index.md): task-oriented `cefari/app`
  usage from frontend code.
- [Notification Behavior](notifications.md): runtime-owned OS notification
  boundary.
