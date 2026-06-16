# CLI Reference

The `cefari` CLI runs, builds, and packages Cefari apps. The top-level surface
is intentionally small: `dev`, `build`, and `package`.

## Installation

Install released CLI builds through npm:

```bash
pnpm add -g cefari
```

The npm distribution provides the Deno-first `cefari` command and the native
desktop runtime helpers needed by Cefari apps.

## Commands

- [Project Commands](project.md): `dev`, `build`, and `package`.
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

## Related App Developer Docs

- [`cefari.config.ts` Reference](../config/index.md)
- [Develop Locally](../guides/development.md)
- [Build And Package](../guides/build-and-package.md)
- [Automated Deployment](../guides/deployment.md)
