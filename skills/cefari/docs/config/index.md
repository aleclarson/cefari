# `cefari.config.ts` Reference

`cefari.config.ts` is the app-developer config file for a Cefari project. The
CLI loads it from the project root for `dev`, `build`, and `package`.

The config file is TypeScript executed through Vite's module runner. It can
default-export an object or a factory function, usually through `defineConfig`:

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
  vite: {
    root: "frontend",
    configFile: "frontend/vite.config.ts",
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

The TypeScript helper provides editor types. The CLI performs runtime validation
after evaluating the config. Legacy `frontend` fields are rejected.

## Sections

- [`app`](app.md): project name, display name, app identifier, and app icon.
- [`capabilities`](capabilities.md): opt-in native desktop integrations.
- [`vite`](frontend.md): Vite root, Vite config file, and development port.
- [`daemon`](daemon.md): Deno daemon entrypoint.
- [`package`](package.md): packaged product name and app version.

## Path Rules

Config paths are resolved relative to the project directory passed to the CLI.
Path fields must stay inside the project and use relative paths. Generated
outputs are written under the project root:

- `build/`
- `dist/`

Keep runtime-maintenance configuration out of `cefari.config.ts`. This file
describes the app project that Cefari commands operate on.
