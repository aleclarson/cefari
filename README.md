# Cefari

Cefari is a Rust desktop app runtime and developer CLI for apps that combine a
web frontend, a Deno daemon, and native OS capabilities.

The repository is split into:

- `cefari-core`: reusable runtime helpers and shared contracts.
- `cefari-desktop`: the shipped desktop runtime.
- `cefari-cli`: the developer tool, exposed as `cefari`.
- `packages/cefari-app`: ergonomic TypeScript wrappers for `window.cefari`.
- `templates/vite-react-basic`: a Deno workspace template using Vite and React.

## Start Here

- [Getting Started](docs/getting-started.md): create, run, build, package, and
  deploy a Cefari app.

## App Developers

- [Scaffold An App](docs/guides/scaffolding.md): create a project and understand
  `cefari.toml`.
- [Develop Locally](docs/guides/development.md): run the frontend, daemon, and
  desktop app together.
- [Build And Package](docs/guides/build-and-package.md): produce local build
  artifacts and package assembly.
- [CLI Reference](docs/cli/index.md): command syntax for development, release,
  and diagnostics.
- [`cefari.toml` Reference](docs/config/index.md): project manifest fields and
  validation rules.
- [Native Capabilities](docs/guides/native-capabilities.md): use Rust-owned
  window, menu, tray, notification, and IPC surfaces.
- [TypeScript App Guide](docs/typescript/index.md): use `@cefari/app` from
  frontend code.
- [Cefari CSS Contract](docs/css-contract.md): opt in to custom titlebar drag
  regions.

## Release Owners

- [Automated Deployment](docs/guides/deployment.md): configure release and
  prerelease workflows.
- [Cefari Release GitHub Action](docs/release-action.md): reference for the
  shared release action.
- [Vite React Template](templates/vite-react-basic/README.md): template-specific
  run, build, and workflow notes.

## Runtime Contributors

- [Architecture Boundary](docs/architecture.md): crate responsibilities and
  dependency boundaries.
- [Cefari IPC Protocol](docs/ipc.md): Specta-generated Rust-to-TypeScript native
  action contract.
- [Runtime Notifications](docs/runtime/notifications.md): notification ownership
  and runtime contributor boundaries.
- [Documentation Verification](docs/verification.md): evidence used to keep the
  docs aligned with the current repository.

## Local Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cefari --help
```
