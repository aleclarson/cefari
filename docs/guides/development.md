# Develop Locally

Run a project in development:

```bash
cefari dev PATH
```

If `PATH` is omitted, Cefari uses the current directory.

## What Dev Mode Starts

`cefari dev` starts:

- a Vite dev server
- the Rust desktop app
- daemon stream support when `daemon.entry` is configured
- configured Deno workers on demand when frontend code calls
  `cefari.workers.spawn()`

When one child process exits or fails, Cefari stops the remaining processes.

Dev mode writes desktop runtime config to `.cefari/config/cefari.json` so the
native runtime can resolve configured workers while loading frontend code from
the Vite dev server.

## Vite Dev Server

Cefari calls Vite's `createServer` API directly:

```ts
vite: {
  root: "frontend",
  configFile: "frontend/vite.config.ts",
  devPort: 5173,
}
```

Override the fixed Vite port from the CLI:

```bash
cefari dev PATH --vite-port 5273
```

For command syntax, see [Project Commands](../cli/project.md). For all
Vite config fields, see [`vite`](../config/frontend.md).

## Vite React Example

The Vite React example project lives at `templates/vite-react-basic/`. It uses
`cefari/app` for typed frontend access to native Cefari actions.

Install workspace dependencies:

```bash
deno install --config templates/vite-react-basic/deno.json
```

Run it with the installed Cefari CLI:

```bash
cefari dev templates/vite-react-basic
```

The example is a Deno workspace with a `frontend/` member. Add a daemon
workspace only when the app needs one.

## Built-In CSS For Custom Titlebars

Cefari injects opt-in drag-region utility classes into trusted main-frame pages.
See [Cefari CSS Contract](../css-contract.md).
