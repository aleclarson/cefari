# cefari

Umbrella package for Cefari app developers. It provides the Deno-first
developer-facing `cefari` command, `cefari.config.ts` helpers, and frontend app
helpers.

The public command surface is intentionally small during Cefari's pre-alpha:

- `cefari dev`
- `cefari build`
- `cefari package`
- `cefari logs`

Release management commands live under `cefari package`.
Log inspection commands live under `cefari logs`.

Use config helpers from the package root:

```ts
import { defineConfig, tray } from "cefari";
```

Use frontend helpers from `cefari/app`:

```ts
import { cefari } from "cefari/app";
```

Use local log helpers from `cefari/logs`.
