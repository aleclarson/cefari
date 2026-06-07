# Vite React Basic

This is a minimal Cefari project template that uses a Deno workspace with Vite and React for the frontend.

Run it from the repository root with the local Cefari build:

```bash
deno install --config templates/vite-react-basic/deno.json
cargo run -p cefari-cli -- dev templates/vite-react-basic
```

Build it with:

```bash
deno install --config templates/vite-react-basic/deno.json
cargo run -p cefari-cli -- build templates/vite-react-basic
```

## Release Workflows

This template includes production and prerelease workflow stubs in `.github/workflows/`.

- `release.yml` runs for `v*` tags or manual dispatch and calls the Cefari release action in production mode.
- `prerelease.yml` runs manually and calls the Cefari release action in prerelease mode, with a dry-run option for validation.

Both workflows use the repository-local action path `./.github/actions/cefari-release` and set `project-path: templates/vite-react-basic` so this checked-in template can be validated against the local Cefari build.

Expected secrets and variables:

- `GITHUB_TOKEN`: provided by GitHub Actions for release creation and asset uploads.
- `UPDATE_SIGNING_KEY`: optional update signing key used when update metadata is generated.
- `CEFARI_UPDATE_URL_BASE`: optional repository variable containing the public download URL prefix for update metadata.

Expected artifacts:

- Native package output under `dist/package/output`.
- GitHub Actions artifact upload named from the Cefari release action's `artifact-name` input.
- Update metadata under `dist/update` when `CEFARI_UPDATE_URL_BASE` and `UPDATE_SIGNING_KEY` are configured.
