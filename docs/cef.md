# CEF Preparation

CEF preparation is a developer and release concern owned by `cefari-cli`, not by `cefari-core`.

Current state:

- Rust crate pin: `cef = "148.4.0"`
- `cefari-desktop` initializes CEF when built with the `cef` feature.
- `cefari-desktop` keeps the `cef` dependency optional until desktop startup and browser loading are verified with packaged resources.
- `cefari build` prepares a CEF metadata directory at `build/cef/`.
- `cefari build` downloads the minimal CEF archive matching `148.0.10`, verifies its SHA1 through `download-cef`, extracts it into `build/cef/resources/`, and records archive metadata in `build/cef/resources/archive.json`.
- `cefari build` keeps downloaded archives and extracted intermediates under `build/cef-cache/`.
- `cefari build` records the pinned Rust CEF version, archive version, host target, downloaded archive name, SHA1, cache directory, and resource directory in `build/cef/manifest.json`.
- `cefari package` reads the downloaded CEF resource directory from the build output and records both the resource directory and `archive.json` path in package metadata and `dist/package/manifest.json`.

Development setup:

```bash
brew install cmake ninja
cargo check -p cefari-desktop --features cef
cefari build PATH
```

The command creates:

- `build/cef/resources/`
- `build/cef/resources/archive.json`
- `build/cef-cache/`
- `build/cef/manifest.json`

Packaging requires these prepared paths to exist. If they are missing, run `cefari build` before `cefari package`.

For deterministic CI and tests, `CEFARI_CEF_RESOURCES_DIR` can point at a pre-populated resources directory that already contains a valid `archive.json`. Normal builds without that override download from `CEF_DOWNLOAD_URL` or the default CEF CDN.

Future CEF work should make CEF non-optional only after desktop startup is verified with those resources.
