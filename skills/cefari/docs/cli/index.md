# CLI Reference

The `cefari` CLI creates, runs, builds, and packages Cefari apps. The top-level
surface is intentionally small: `init`, `dev`, `build`, and `package`.

## Installation

Install released CLI builds through npm:

```bash
pnpm add -g cefari
```

The npm distribution provides the TypeScript/Node `cefari` command and the
native desktop runtime helpers needed by Cefari apps.

## Commands

- [Project Commands](project.md): `init`, `dev`, `build`, and `package`.
- [Release Commands](release.md): `package sign`, `package notarize`, and
  `package update`.

## Project Path Defaults

Most project commands accept an optional path. When omitted, Cefari uses the
current directory:

```bash
cefari dev
cefari build
cefari package
```

`cefari init` is different: when omitted, its path defaults to `./cefari-app`.

## Related App Developer Docs

- [`cefari.config.ts` Reference](../config/index.md)
- [Scaffold An App](../guides/scaffolding.md)
- [Develop Locally](../guides/development.md)
- [Build And Package](../guides/build-and-package.md)
- [Automated Deployment](../guides/deployment.md)
