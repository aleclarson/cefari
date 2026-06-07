# CEF Preparation

CEF preparation is a developer and release concern owned by `cefari-cli`, not by `cefari-core`.

Current state:

- Rust crate pin: `cef = "148.4.0"`
- `cefari-desktop` keeps the `cef` dependency optional until initialization is implemented and verified.
- `cefari build` prepares a CEF metadata directory at `build/cef/`.
- `cefari build` records the pinned CEF version, host target, source state, and resource directory in `build/cef/manifest.json`.
- `cefari build` does not perform the large CEF binary download yet; the preparation manifest records `source = "pending-download"`.
- `cefari package` reads the prepared CEF resource directory from the build output and records it in package metadata and `dist/package/manifest.json`.

Development setup:

```bash
cefari build PATH
```

The command creates:

- `build/cef/resources/`
- `build/cef/manifest.json`

Packaging requires these prepared paths to exist. If they are missing, run `cefari build` before `cefari package`.

Future CEF preparation should:

- select the CEF binary distribution compatible with the pinned `cef` crate
- download into a cache directory ignored by git
- make CEF initialization non-optional only after desktop startup is verified with those resources
