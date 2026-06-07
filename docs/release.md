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

CEF resources are still recorded as external in the package manifest until the CLI-owned CEF preparation step is implemented.

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
