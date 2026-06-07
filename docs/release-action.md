# Cefari Release GitHub Action

The Cefari release action lives at `.github/actions/cefari-release` and is intended to be the shared release entry point for Cefari app templates.

## Interface

The action accepts a Cefari project path, release mode, target platforms, signing/notarization options, update metadata inputs, and artifact upload controls. The full machine-readable contract is in `.github/actions/cefari-release/action.yml`.

Required input:

- `release-version`: version advertised in package and update metadata.

Common optional inputs:

- `project-path`: path to the app project. Defaults to `.`.
- `mode`: `release` or `prerelease`. Defaults to `release`.
- `targets`: comma-separated target platform list.
- `release-tag`: Git tag used for GitHub release assets.
- `create-github-release`: whether to create or update a GitHub release.
- `upload-artifacts`: whether to upload packaged files to GitHub Actions artifacts.

Signing and notarization inputs:

- `signing-platform`: platform passed to `cefari codesign`.
- `signing-config`: optional path to `sign.toml`.
- `notarize`: whether macOS notarization should run after signing.

Update inputs:

- `update-url-base`: public download URL prefix for generated update metadata.
- `update-target`: updater target key.
- `update-format`: updater package format.
- `update-key-env`: environment variable containing the update signing key. Defaults to `UPDATE_SIGNING_KEY`.

## Secret-Dependent Behavior

Implementation should skip signing, notarization, and update signing when the required secrets or config are absent and the corresponding behavior was not explicitly requested. If a caller explicitly requests one of those steps, missing credentials should fail with a clear message.

Expected secret families:

- Code-signing certificates and passwords for platform signing.
- Apple notarization credentials for macOS notarization.
- `UPDATE_SIGNING_KEY` or the env var named by `update-key-env` for update metadata signing.
- `GITHUB_TOKEN` permissions for creating GitHub releases and uploading release assets.

## Outputs

- `package-dir`: prepared package assembly directory.
- `update-dir`: generated update metadata directory.
- `artifact-dir`: upload-ready artifact directory.
- `release-mode`: effective release mode.

## Implementation Boundary

The first implementation should delegate build, package, signing, notarization, and update generation to `cefari-cli` commands instead of duplicating release logic in workflow YAML.
