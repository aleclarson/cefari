# Vite React Basic

This is a minimal Cefari project template that uses a Deno workspace with Vite
and React for the frontend.

The frontend imports `@cefari/app` for ergonomic wrappers over `window.cefari`.
In ordinary browser preview calls reject with a typed unsupported Cefari error;
inside trusted Cefari pages they use the native bridge.

For custom titlebars, apply `cefari-drag` only to regions that should move the
window. Use `cefari-no-drag` on interactive descendants that must keep normal
pointer behavior.

Run it from the repository root with an installed `cefari` CLI:

```bash
deno install --config templates/vite-react-basic/deno.json
cefari dev templates/vite-react-basic
```

Build it with:

```bash
deno install --config templates/vite-react-basic/deno.json
cefari build templates/vite-react-basic
```

## Release Workflows

This template includes production and prerelease workflow stubs in
`.github/workflows/`.

- `release.yml` runs for `v*` tags or manual dispatch and calls the Cefari
  release action in production mode.
- `prerelease.yml` runs manually and calls the Cefari release action in
  prerelease mode, with a dry-run option for validation.

Both workflows use the repository-local action path
`./.github/actions/cefari-release` and set
`project-path: templates/vite-react-basic` so this checked-in template can be
validated with the installed `cefari` CLI.

Expected secrets and variables:

- `GITHUB_TOKEN`: provided by GitHub Actions for release creation and asset
  uploads.
- `UPDATE_SIGNING_KEY`: optional update signing key used when update metadata is
  generated.
- `CEFARI_UPDATE_URL_BASE`: optional repository variable containing the public
  download URL prefix for update metadata.

Expected artifacts:

- Native package output under `dist/package/output`.
- GitHub Actions artifact upload named from the Cefari release action's
  `artifact-name` input.
- Update metadata under `dist/update` when `CEFARI_UPDATE_URL_BASE` and
  `UPDATE_SIGNING_KEY` are configured.
