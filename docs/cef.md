# CEF Preparation

CEF preparation is a developer and release concern owned by `cefari-cli`, not by `cefari-core`.

Current state:

- Rust crate pin: `cef = "148.4.0"`
- `cefari-desktop` keeps the `cef` dependency optional until initialization is implemented and verified.
- `cefari build` does not download CEF yet.
- `cefari package` records CEF resources as pending external resources in the package assembly manifest.

Future CEF preparation should:

- select the CEF binary distribution compatible with the pinned `cef` crate
- download into a cache directory ignored by git
- expose clear errors when CEF resources are missing during packaging
- make CEF initialization non-optional only after desktop startup is verified with those resources

