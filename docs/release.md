# Release Workflows

Cefari release work is split between generated project artifacts, external packaging/signing tools, and CI.

## Packaging

Run a build before packaging:

```bash
cefari build PATH
cefari package PATH
```

`cefari package` writes package assembly metadata to `dist/package/`:

- `cargo-packager.toml`
- `manifest.json`

When `cargo-packager` is available on `PATH`, the CLI invokes it with the generated config and writes native package output to `dist/package/output/`. When it is missing, the metadata still remains available for CI or later local packaging.

CEF resources are resolved from `build/cef/resources/`, which is prepared by `cefari build` and recorded in `build/cef/manifest.json`. The current manifest still marks the source as `pending-download` until the large CEF binary download step is implemented.

CI runs package assembly on macOS, Linux, and Windows. Those jobs create a sample Cefari project, run `cefari build`, run `cefari package`, verify the generated frontend, daemon, CEF preparation, package metadata, and confirm the `cefari` CLI binary is built separately from desktop package metadata.

Daemon package inputs include the copied source entry and the compiled `cefari-daemon` executable produced by `deno compile`; package metadata records the executable path explicitly.

Native installer validation is still separate from package assembly validation and remains blocked on downloaded CEF binaries and real package contents.

## Signing

Sign a packaged artifact with:

```bash
cefari codesign ARTIFACT
```

The command delegates signing to `cargo-codesign`. Use `--config PATH` to point at a `sign.toml` file when auto-discovery is not enough.

macOS `.app` and `.dmg` artifacts are signed with notarization skipped so signing and notarization can be run as separate release steps.

## Notarization

Notarize macOS artifacts with:

```bash
cefari notarize ARTIFACT
```

The artifact must be a `.app` bundle or `.dmg` file. Credentials and signing identities are supplied through `cargo-codesign` configuration or environment variables.

## Update Artifacts

Generate updater metadata with:

```bash
cefari make-update ARCHIVE --url URL --version VERSION
```

The command signs `ARCHIVE` through `cargo-codesign codesign update`, then writes `dist/update/update.json`. The JSON follows the response shape consumed by `cargo-packager-updater`: a release `version` with per-target `url` and `signature` entries.

Use `--target`, `--key-env`, and `--output-dir` when CI needs explicit platform keys, signing-key environment names, or output locations.
