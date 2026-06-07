# Cefari CLI

`cefari-cli` builds the `cefari` developer tool. It is distributed separately from the shipped desktop app.

## Commands

### `cefari init [PATH] [--name NAME]`

Creates a Cefari project at `PATH`. If `PATH` is omitted, the CLI creates `./cefari-app`.

Generated files:

- `cefari.toml`
- `frontend/index.html`
- `daemon/main.ts`
- `README.md`

The generated `cefari.toml` currently contains:

- `[app]` name and identifier
- `[frontend]` dist path
- `[daemon]` entry path
- `[package]` product name

The command refuses to overwrite an existing path.

### `cefari doctor`

Prints local tool availability for:

- `cargo`
- `deno`
- `cargo-packager`
- `cargo-codesign`

Missing tools are reported as `missing`; the command does not install them.

### `cefari info`

Prints the CLI version, target OS, target architecture, and project details when run from a directory containing `cefari.toml`.

### Planned Commands

These commands are present in the parser but intentionally fail until their orchestration work is implemented:

- `cefari dev`
- `cefari build`
- `cefari package`
- `cefari codesign`
- `cefari notarize`
- `cefari make-update`

## Troubleshooting `cefari doctor`

- `cargo: missing` means Rust or Cargo is not on `PATH`.
- `deno: missing` means daemon development/build commands will not be able to run Deno until it is installed and on `PATH`.
- `cargo-packager: missing` means `cefari package` will not be able to package apps until the tool is installed or provided by CI.
- `cargo-codesign: missing` means signing commands will not be able to run until the tool is installed or provided by CI.

