# Vite React Basic

This is a minimal Cefari project template for a Vite app with a native Cefari
desktop shell, packaging, updater support, and desktop capabilities. It does
not configure a daemon by default.

The frontend imports `cefari/app` for ergonomic wrappers over `window.cefari`.
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

To override the fixed Vite development port:

```bash
cefari dev templates/vite-react-basic --vite-port 5173
```

Build it with:

```bash
deno install --config templates/vite-react-basic/deno.json
cefari build templates/vite-react-basic
```

Package it with:

```bash
cefari package templates/vite-react-basic
```

## Optional Daemon

Add a daemon only when your app needs one:

```ts
daemon: {
  entry: "daemon/main.ts",
}
```

Daemon stdout is reserved for byte-stream protocol data when frontend code uses
`cefari.daemon.connect()`. Write daemon logs to stderr.

## Release Workflows

This template includes production and prerelease workflow stubs in
`.github/workflows/`.

- `release.yml` runs for `v*` tags or manual dispatch, installs release
  prerequisites, and calls the Cefari release action in production mode across
  macOS, Linux, and Windows matrix jobs.
- `prerelease.yml` runs manually and calls the Cefari release action in
  prerelease mode across the same matrix, with a dry-run option for validation.

Both workflows use the repository-local action path
`./.github/actions/cefari-release` and set
`project-path: templates/vite-react-basic` so this checked-in template can be
validated in this repository. Generated app repositories should usually change
`project-path` to `.`.

Expected secrets and variables:

- `GITHUB_TOKEN`: provided by GitHub Actions for release creation and asset
  uploads.
- `UPDATE_SIGNING_KEY`: optional update signing key used when update metadata is
  generated.
- `CEFARI_UPDATE_URL_BASE`: optional repository variable containing the public
  download URL prefix for update metadata.
- `CEFARI_CLI_VERSION`: required repository variable pinning the `cefari`
  version installed by the release action.

Expected artifacts:

- Native package output under `dist/package/output`.
- GitHub Actions artifact upload named from the Cefari release action's
  `artifact-name` input.
- Release artifact list at `dist/release-artifacts.txt`.
- Per-target update input archive under `dist/update-input/`.
- Update metadata under `dist/update` when `CEFARI_UPDATE_URL_BASE` and
  `UPDATE_SIGNING_KEY` are configured.
