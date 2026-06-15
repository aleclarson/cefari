# `cefari.config.ts` Reference

`cefari.config.ts` is the app-developer config file for a Cefari project. The
CLI loads it from the project root for `dev`, `build`, `package`, `clean`, and
`info`.

The config file is TypeScript executed by Deno. It should default-export a
JSON-serializable object, usually through `defineConfig`:

```ts
import { defineConfig, tray } from "@cefari/cli";

export default defineConfig({
  app: {
    projectName: "my-cefari-app",
    name: "My Cefari App",
    identifier: "dev.cefari.my-cefari-app",
  },
  capabilities: [
    tray({
      icon: "assets/tray-icon.png",
    }),
  ],
  frontend: {
    dist: "frontend/dist",
    devPort: 5173,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "My Cefari App",
    version: "0.1.0",
  },
});
```

The TypeScript helper provides editor types only. The CLI performs runtime
validation after Deno evaluates the config. Unknown fields, missing required
fields, wrong types, invalid names, invalid versions, invalid paths, invalid
ports, and malformed command arrays are rejected before commands use the config.

## Sections

- [`app`](app.md): project name, display name, app identifier, and app icon.
- [`capabilities`](capabilities.md): opt-in native desktop integrations.
- [`frontend`](frontend.md): frontend dist path and optional build/dev commands.
- [`daemon`](daemon.md): Deno daemon entrypoint.
- [`package`](package.md): packaged product name and app version.

## Deno

Cefari expects Deno `2.8+` to execute `cefari.config.ts`. Missing Deno is an
error. Older installed Deno versions produce a warning and continue.

Config execution is granted read access to the project root and Cefari's
temporary loader module plus environment-variable access. Network, write, and
subprocess permissions are not granted by default.

## Path Rules

Config paths are resolved relative to the project directory passed to the CLI.
Path fields must stay inside the project and use relative paths. Generated
outputs are written under the project root:

- `build/`
- `dist/`

Keep runtime-maintenance configuration out of `cefari.config.ts`. This file
describes the app project that Cefari commands operate on.
