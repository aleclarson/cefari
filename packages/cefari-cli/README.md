# cefari

Umbrella package for Cefari app developers. It provides the developer-facing
`cefari` command, `cefari.config.ts` helpers, and frontend app helpers.

The public command surface is intentionally small during Cefari's pre-alpha:

- `cefari init`
- `cefari dev`
- `cefari build`
- `cefari package`

Release management commands live under `cefari package`.

Use config helpers from the package root:

```ts
import { defineConfig, tray } from "cefari";
```

Use frontend helpers from `cefari/app`:

```ts
import { cefari } from "cefari/app";
```
