# CLI Reference

The `cefari` CLI creates, runs, builds, packages, signs, and inspects Cefari app
projects. These pages are app-developer command references.

## Installation

Install released CLI builds through npm:

```bash
npm install -g @cefari/cli
```

The npm distribution bundles the native `cefari` binary and matching
`cefari-desktop` runtime for supported macOS arm64, macOS x64, Linux x64, and
Windows x64 hosts.

## Commands

- [Project Commands](project.md): `init`, `dev`, `build`, `package`, and
  `clean`.
- [Release Commands](release.md): `codesign`, `notarize`, and `make-update`.
- [Diagnostics Commands](diagnostics.md): `doctor`, `info`, and `logs`.

## Project Path Defaults

Most project commands accept an optional path. When omitted, Cefari uses the
current directory:

```bash
cefari dev
cefari build
cefari package
cefari clean
```

`cefari init` is different: when omitted, its path defaults to `./cefari-app`.

## Related App Developer Docs

- [`cefari.toml` Reference](../config/index.md)
- [Scaffold An App](../guides/scaffolding.md)
- [Develop Locally](../guides/development.md)
- [Build And Package](../guides/build-and-package.md)
- [Automated Deployment](../guides/deployment.md)
