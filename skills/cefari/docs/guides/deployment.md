# Automated Deployment

Cefari release automation is split between the local CLI, a composite GitHub
Action, and template workflow files.

## Shared Release Action

The shared action lives at:

```text
.github/actions/cefari-release/action.yml
```

It validates release inputs, then delegates to the configured Cefari CLI command
for build, package, signing, notarization, update metadata, and GitHub release
upload.

Common inputs:

- `project-path`
- `mode`: `release` or `prerelease`
- `targets`: informational current-runner target label; use workflow matrices
  for platform fan-out
- `cefari-command`
- `install-cli`
- `cefari-version`
- `release-tag`
- `create-github-release`
- `upload-artifacts`
- `dry-run`
- `artifact-name`
- `release-version`: optional override for `[package].version`

Signing and update inputs:

- `signing-platform`
- `signing-config`
- `notarize`
- `update-url-base`
- `update-target`
- `update-format`
- `update-key-env`

The full machine-readable contract is the action metadata file.

## Template Workflows

The Vite React template includes:

- `templates/vite-react-basic/.github/workflows/release.yml`
- `templates/vite-react-basic/.github/workflows/prerelease.yml`

The production workflow runs for `v*` tags and manual dispatch. The prerelease
workflow runs manually and includes a `dry_run` input.

Both workflows run a matrix across macOS, Linux, and Windows, install Node,
Deno, Rust, platform CEF build tools, `cargo-packager`, and `cargo-codesign`,
then call the repository-local Cefari release action with the current matrix
target:

```yaml
project-path: templates/vite-react-basic
targets: ${{ matrix.target.name }}
install-cli: "true"
cefari-version: ${{ vars.CEFARI_CLI_VERSION }}
update-target: ${{ matrix.target.name }}
```

Copy these workflow files and `.github/actions/cefari-release/` into a generated
project, then change `project-path` to `.`, adjust workflow matrix targets,
set the `CEFARI_CLI_VERSION` repository variable, review signing configuration,
and set update URLs for that repository.

## Dry-Run A Release

Use the prerelease workflow's `dry_run` input, or run the action script locally
with action-like environment variables:

```bash
GITHUB_OUTPUT=/tmp/cefari-release-output.txt \
GITHUB_ACTION_PATH=.github/actions/cefari-release \
CEFARI_PROJECT_PATH=templates/vite-react-basic \
CEFARI_RELEASE_MODE=prerelease \
CEFARI_COMMAND=cefari \
CEFARI_INSTALL_CLI=false \
CEFARI_RELEASE_VERSION=0.0.0-verification \
CEFARI_RELEASE_TAG=verification-0.0.0 \
CEFARI_DRY_RUN=true \
deno run \
  --config .github/actions/cefari-release/deno.json \
  --allow-read \
  --allow-write \
  --allow-env \
  --allow-run \
  .github/actions/cefari-release/src/main.ts
```

Dry-run mode prints the planned build, package, signing, notarization, update,
and GitHub release commands without executing the release steps.

## Signing And Notarization

Sign a packaged artifact:

```bash
cefari package sign ARTIFACT
```

Notarize a signed macOS `.app` bundle or `.dmg`:

```bash
cefari package notarize ARTIFACT
```

Use `--config PATH` to point at a `sign.toml` when the signing tool needs
explicit configuration.

## Update Metadata

Generate update metadata:

```bash
cefari package update ARCHIVE --url URL --version VERSION
```

Useful options:

- `--target TARGET`
- `--format FORMAT`
- `--key-env NAME`
- `--output-dir PATH`

The default update signing key environment variable is `UPDATE_SIGNING_KEY`.

For command syntax and defaults, see [Release Commands](../cli/release.md).

## Required CI State

Release jobs that publish real artifacts need:

- `GITHUB_TOKEN` permissions for release and artifact operations
- signing identities and passwords for platform signing
- Apple notarization credentials for macOS notarization
- `UPDATE_SIGNING_KEY` or the env var named by `update-key-env` when update
  metadata is generated
