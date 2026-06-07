# Cefari

Cefari is a Rust desktop app runtime and developer CLI for apps that combine a web frontend, a Deno daemon, and native OS capabilities.

The repository is split into:

- `cefari-core`: reusable runtime helpers and shared contracts.
- `cefari-desktop`: the shipped desktop runtime.
- `cefari-cli`: the developer tool, exposed as `cefari`.
- `templates/vite-react-basic`: a Deno workspace template using Vite and React.

## Start Here

- [Getting Started](docs/getting-started.md): create, run, build, package, and deploy a Cefari app.

## App Developers

- [Scaffold An App](docs/guides/scaffolding.md): create a project and understand `cefari.toml`.
- [Develop Locally](docs/guides/development.md): run the frontend, daemon, and desktop app together.
- [Build And Package](docs/guides/build-and-package.md): produce local build artifacts and package assembly.
- [Native Capabilities](docs/guides/native-capabilities.md): use Rust-owned window, menu, tray, notification, and IPC surfaces.
- [Cefari CSS Contract](docs/css-contract.md): opt in to custom titlebar drag regions.

## Release Owners

- [Automated Deployment](docs/guides/deployment.md): configure release and prerelease workflows.
- [Cefari Release GitHub Action](docs/release-action.md): reference for the shared release action.
- [Vite React Template](templates/vite-react-basic/README.md): template-specific run, build, and workflow notes.

## Runtime Contributors

- [Architecture Boundary](docs/architecture.md): crate responsibilities and dependency boundaries.
- [Cefari IPC Protocol](docs/ipc.md): Specta-generated Rust-to-TypeScript native action contract.
- [Desktop Notifications](docs/notifications.md): OS notification ownership and runtime behavior.
- [Documentation Verification](docs/verification.md): evidence used to keep the docs aligned with the current repository.

## Local Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cefari-cli -- --help
```
