# Release Workflows

Cefari release work is split between generated project artifacts, external packaging/signing tools, and CI.

## Packaging

Run a build before packaging:

```bash
cefari build PATH
cefari package PATH
```

For release artifacts, use the Cargo release profile:

```bash
cefari build PATH --release
cefari package PATH --release
```

`cefari package` writes package assembly metadata to `dist/package/`:

- `cargo-packager.toml`
- `manifest.json`

When `cargo-packager` is available on `PATH`, the CLI invokes it with the generated config and writes native package output to `dist/package/output/`. When it is missing, the metadata still remains available for CI or later local packaging.

CEF resources are resolved from `build/cef/resources/`, which is populated by `cefari build` using the minimal CEF archive selected through `download-cef`. `build/cef/resources/archive.json` records the downloaded archive name and SHA1, and `build/cef/manifest.json` records the archive version, host target, cache directory, and resource directory.

CI runs package assembly on macOS, Linux, and Windows. Those jobs create a sample Cefari project, run `cefari build`, run `cefari package`, verify the generated frontend, daemon, CEF preparation, package metadata, and confirm the `cefari` CLI binary is built separately from desktop package metadata.

Daemon package inputs include the copied source entry and the compiled `cefari-daemon` executable produced by `deno compile`; package metadata records the executable path explicitly. CEF package inputs include the downloaded resource directory and verified `archive.json`.

The package assembly jobs build the `cefari` CLI in release mode, verify `cefari --version`, and copy the binary into a separate CI distribution directory. Package manifest checks assert that desktop app metadata names `cefari-desktop` and does not include the CLI distribution path.

Native installer validation is separate from package assembly validation. Release jobs extract or inspect platform package outputs before upload and require the packaged payload to contain `cefari-desktop`, generated frontend files, generated daemon output, CEF archive metadata, and CEF payload resources.

Release tags and manual dispatches run `.github/workflows/release.yml`. That workflow builds native packages on macOS, Linux, and Windows using real `cefari build --release` CEF preparation, invokes `cefari package --release`, and uploads each platform's package output as a workflow artifact. Signing runs when `CEFARI_ENABLE_SIGNING` is set to `true` in repository secrets. macOS notarization runs when `CEFARI_ENABLE_NOTARIZATION` is set to `true`.

Before upload, the release workflow verifies that native package output exists and that package metadata points at existing frontend, daemon executable, CEF resource directory, and CEF archive metadata inputs.

### Local macOS release smoke

On macOS, a full local release smoke can verify the real downloaded CEF payload and release-profile desktop binary:

```bash
cefari init /tmp/cefari-real-release-smoke --name "Real Release Smoke"
cefari build /tmp/cefari-real-release-smoke --release
PATH="$HOME/.cargo/bin:$PATH" cefari package /tmp/cefari-real-release-smoke --release
```

Inspect `dist/package/output/*.app` and `dist/package/output/*.dmg` to confirm the `.app` contains `Contents/MacOS/cefari-desktop`, `Contents/Resources/frontend/index.html`, `Contents/Resources/daemon/cefari-daemon`, `Contents/Resources/cef/archive.json`, and additional CEF payload files. `scripts/verify-native-package-payload.rb INSPECT_DIR macOS` performs the payload file checks. `cargo tree -p cefari-desktop -i cefari-core` confirms the packaged desktop binary is built from the crate that links `cefari-core`.

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

The command signs `ARCHIVE` through `cargo-codesign codesign update`, then writes `dist/update/update.json`. The JSON follows the response shape consumed by `cargo-packager-updater`: a release `version` with per-target `url`, `signature`, and `format` entries.

Use `--target`, `--format`, `--key-env`, and `--output-dir` when CI needs explicit platform keys, package formats, signing-key environment names, or output locations.

The release workflow downloads the native package artifacts, archives each platform package, runs `cefari make-update` for each platform target, uploads generated update metadata, and publishes native packages, update archives, signatures, and `update.json` files to the GitHub release for tag builds. `UPDATE_SIGNING_KEY` must be configured as a repository secret for update artifact generation.
