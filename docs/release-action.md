# Cefari Release GitHub Action

The Cefari release action lives at `.github/actions/cefari-release` and is intended to be the shared release entry point for Cefari app templates.

For step-by-step setup, see [Automated Deployment](guides/deployment.md).

## Interface

The action accepts a Cefari project path, release mode, current-runner target
label, CLI setup options, signing/notarization options, update metadata inputs,
and artifact upload controls. The full machine-readable contract is in
`.github/actions/cefari-release/action.yml`.

Required input:

- `release-version`: version advertised in package and update metadata.

Common optional inputs:

- `project-path`: path to the app project. Defaults to `.`.
- `mode`: `release` or `prerelease`. Defaults to `release`.
- `targets`: informational current-runner target label. Use a workflow matrix
  for platform fan-out.
- `cefari-command`: Cefari CLI command or path. Defaults to `cefari`.
- `install-cli`: whether to install `@cefari/cli` from npm before release
  commands. Defaults to `false`.
- `cefari-version`: npm CLI version to install when `install-cli` is `true`.
  Defaults to `latest`.
- `release-tag`: Git tag used for GitHub release assets.
- `create-github-release`: whether to create or update a GitHub release.
- `upload-artifacts`: whether to upload packaged files to GitHub Actions artifacts.
- `dry-run`: validate inputs and print planned commands without running build, package, signing, release, or upload commands.

Signing and notarization inputs:

- `signing-platform`: platform passed to `cefari codesign`.
- `signing-config`: optional path to `sign.toml`.
- `notarize`: whether macOS notarization should run after signing.

Update inputs:

- `update-url-base`: public download URL prefix for generated update metadata.
- `update-target`: updater target key. Defaults to the current `targets`
  value, or an inferred current-runner target when `targets` is omitted.
- `update-format`: updater package format.
- `update-key-env`: environment variable containing the update signing key. Defaults to `UPDATE_SIGNING_KEY`.

When update generation runs, the action creates a deterministic per-target
archive under `dist/update-input/<target>.zip`, signs that archive with
`cefari make-update`, and includes generated update files in the GitHub release
upload set.

## Secret-Dependent Behavior

The implementation skips signing when no signing platform or signing config is provided, skips notarization unless `notarize` is `true`, and skips update metadata unless `update-url-base` is provided. If update metadata is requested but the env var named by `update-key-env` is absent, update generation is skipped with a clear log message. GitHub release creation requires `gh` when `create-github-release` is `true`.
When GitHub release creation is enabled, `GH_TOKEN` or `GITHUB_TOKEN` must be
available. The action creates the release if it does not exist, otherwise it
uploads to the existing release with `--clobber`. Directory artifacts such as
macOS `.app` bundles are archived before upload.

## CLI Setup

By default the action expects a `cefari` command to already be available on
`PATH`. Generated app repositories can either install the npm CLI before
calling the action or set:

```yaml
with:
  install-cli: "true"
  cefari-version: "latest"
```

When `install-cli` is `true`, the action installs `@cefari/cli` from npm and
then runs release commands through the configured `cefari-command`.
Dry runs print the planned install and release commands without requiring the
CLI to exist. Non-dry runs fail early when `npm` or the configured Cefari
command is unavailable.

Expected secret families:

- Code-signing certificates and passwords for platform signing.
- Apple notarization credentials for macOS notarization.
- `UPDATE_SIGNING_KEY` or the env var named by `update-key-env` for update metadata signing.
- `GITHUB_TOKEN` permissions for creating GitHub releases and uploading release assets.

## Outputs

- `package-dir`: prepared package assembly directory.
- `update-dir`: generated update metadata directory.
- `artifact-dir`: upload-ready artifact directory.
- `release-artifacts`: newline-delimited file listing package artifacts
  collected for release processing.
- `release-mode`: effective release mode.

## Implementation Boundary

The implementation delegates build, package, signing, notarization, and update
generation to the configured Cefari CLI command instead of duplicating release
logic in workflow YAML.
