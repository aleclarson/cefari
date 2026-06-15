# Contributing

This guide documents the local shell workflow for improving Cefari itself. It
focuses on commands run from a clone of this repository.

## Clone The Repository

```bash
git clone https://github.com/cefari/cefari.git
cd cefari
git status --short
```

Create a topic branch for each change:

```bash
git switch -c codex/my-change
```

## Install Local Tools

Cefari is a Rust workspace with a pnpm-managed TypeScript package and Deno
templates. Use a current stable Rust toolchain with Rust 1.85 or newer,
Node.js 22 or newer, pnpm 11.5.1, and Deno 2.x.

On macOS, install the native build tools used by CI:

```bash
brew install cmake ninja
```

On Linux, install desktop build libraries before building the desktop runtime:

```bash
sudo apt-get update
sudo apt-get install -y cmake ninja-build libgtk-3-dev libayatana-appindicator3-dev libxdo-dev
```

Install optional packaging tools when working on package or release behavior:

```bash
cargo install cargo-packager --version 0.11.8 --locked
cargo install cargo-codesign --version 0.4.2 --locked
```

Check the local environment with the CLI:

```bash
node npm/dist/bin/cefari.js --help
node npm/dist/bin/cefari.js package
```

## Build And Run Cefari

Build the Rust workspace:

```bash
cargo build --workspace
```

Build only the CLI:

```bash
pnpm --dir npm build
```

Run the CLI from source:

```bash
node npm/dist/bin/cefari.js --help
node npm/dist/bin/cefari.js init .tmp/cefari-sample --name "Cefari Sample"
node npm/dist/bin/cefari.js dev .tmp/cefari-sample
```

Install the local CLI when you want to use `cefari` directly while testing
projects or templates:

```bash
pnpm add -g ./npm
cefari --help
```

Run the checked-in Vite React template:

```bash
deno install --config templates/vite-react-basic/deno.json
cefari dev templates/vite-react-basic
```

Use a different frontend port when the default is occupied:

```bash
cefari dev templates/vite-react-basic --vite-port 5273
```

## Common Edit Loops

Format Rust code:

```bash
cargo fmt --all
```

Check Rust formatting without changing files:

```bash
cargo fmt --all --check
```

Run Rust checks:

```bash
cargo check --workspace
cargo check -p cefari-desktop
```

Run Rust tests:

```bash
cargo test --workspace
cargo test -p cefari-core
pnpm --dir npm test
cargo test -p cefari-desktop
cargo test -p cefari-core services::tests -- --nocapture
```

Run Clippy with the same warning policy as CI:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Run TypeScript package checks:

```bash
pnpm --dir npm check
pnpm --dir npm test
```

Check the template workspace:

```bash
deno install --config templates/vite-react-basic/deno.json
deno task --config templates/vite-react-basic/deno.json build:frontend
```

## Verify With The Vite React Template

Use `templates/vite-react-basic/` as the first real-project check for changes
that affect app scaffolding, `cefari.config.ts`, frontend dev servers, daemon
startup, desktop runtime behavior, IPC, packaging, release assembly, or
`cefari/app`. The template is checked into this repository, so it exercises the
same project shape users get without requiring a separate app clone.

Install its Deno workspace dependencies:

```bash
deno install --config templates/vite-react-basic/deno.json
```

Run its frontend build directly when your change touches template frontend code,
the Deno workspace, or `cefari/app` imports:

```bash
deno task --config templates/vite-react-basic/deno.json build:frontend
```

Run it through the local Cefari CLI when your change touches `cefari dev`, the
desktop shell, daemon launch, native bridge behavior, ports, or the development
origin policy:

```bash
node npm/dist/bin/cefari.js dev templates/vite-react-basic
```

If you installed the local CLI, the equivalent command is:

```bash
cefari dev templates/vite-react-basic
```

Build and package the template when your change touches build output, daemon
compilation, CEF resource handling, package metadata, or release assembly:

```bash
node npm/dist/bin/cefari.js build templates/vite-react-basic
node npm/dist/bin/cefari.js package templates/vite-react-basic
```

Use release mode for packaging changes that depend on optimized binaries:

```bash
node npm/dist/bin/cefari.js build templates/vite-react-basic --release
node npm/dist/bin/cefari.js package templates/vite-react-basic --release
```

Remove generated template artifacts after verification:

```bash
rm -rf templates/vite-react-basic/build templates/vite-react-basic/dist
```

When Rust IPC types change, keep the generated TypeScript declarations in sync:

```bash
cargo test -p cefari-core generated_typescript_bindings_are_current
cp crates/cefari-core/bindings/ipc.ts npm/src/app/ipc.ts
pnpm --dir npm check
pnpm --dir npm test
```

## Build And Package Smoke Tests

Create a disposable Cefari app:

```bash
rm -rf .tmp/cefari-sample
node npm/dist/bin/cefari.js init .tmp/cefari-sample --name "Cefari Sample"
```

Build it with the debug profile:

```bash
node npm/dist/bin/cefari.js build .tmp/cefari-sample
```

Package it:

```bash
node npm/dist/bin/cefari.js package .tmp/cefari-sample
```

Build and package with the release profile:

```bash
node npm/dist/bin/cefari.js build .tmp/cefari-sample --release
node npm/dist/bin/cefari.js package .tmp/cefari-sample --release
```

Use a fixture CEF resources directory when you need deterministic package
assembly without downloading CEF resources:

```bash
mkdir -p .tmp/cef-fixture
printf '%s\n' '{"type":"minimal","name":"cef_binary_fixture.tar.bz2","sha1":"fixture-sha1"}' > .tmp/cef-fixture/archive.json
printf '%s\n' fixture > .tmp/cef-fixture/libcef.fixture
CEFARI_CEF_RESOURCES_DIR=.tmp/cef-fixture node npm/dist/bin/cefari.js build .tmp/cefari-sample
node npm/dist/bin/cefari.js package .tmp/cefari-sample
```

Inspect generated package payloads with the repository scripts:

```bash
scripts/extract-native-package-payload.sh .tmp/cefari-sample/dist/package/output .tmp/package-inspect
ruby scripts/verify-native-package-payload.rb .tmp/package-inspect "$(uname -s)"
```

Remove generated app artifacts:

```bash
rm -rf .tmp
```

## Before Opening A Pull Request

Run the main local checks:

```bash
cargo fmt --all --check
cargo check --workspace
cargo check -p cefari-desktop
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm --dir npm check
pnpm --dir npm test
```

Review the exact changes:

```bash
git status --short
git diff
```

Commit focused changes with a Conventional Commit message:

```bash
git add CONTRIBUTING.md
git commit -m "docs: add local contribution workflow"
```

Push the branch:

```bash
git push -u origin HEAD
```
