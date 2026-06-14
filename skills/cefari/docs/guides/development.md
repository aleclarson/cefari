# Develop Locally

Run a project in development:

```bash
cefari dev PATH
```

If `PATH` is omitted, Cefari uses the current directory.

## What Dev Mode Starts

`cefari dev` starts:

- a frontend dev server
- the Deno daemon entry with watch mode
- the Rust desktop app

When one child process exits or fails, Cefari stops the remaining processes.

## Frontend Dev Server

Without a configured frontend dev command, Cefari serves `frontend/index.html`
with a built-in local static server.

Projects can configure a tool-managed frontend dev server:

```ts
frontend: {
  dist: "frontend/dist",
  devCommand: ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"],
  devPort: 5173,
}
```

`{port}` is replaced with the selected frontend port. Override it from the CLI:

```bash
cefari dev PATH --frontend-port 5273
```

Use `--frontend-port 0` only with the built-in static server, where Cefari can
bind an available local port itself.

For command syntax, see [Project Commands](../cli/project.md). For all
frontend config fields, see [`frontend`](../config/frontend.md).

## Vite React Example

The Vite React example project lives at `templates/vite-react-basic/`. It uses
`@cefari/app` for typed frontend access to native Cefari actions.

Install workspace dependencies:

```bash
deno install --config templates/vite-react-basic/deno.json
```

Run it with the installed Cefari CLI:

```bash
cefari dev templates/vite-react-basic
```

The example is a Deno workspace with `frontend/` and `daemon/` members. Its
Cefari config uses `deno task` commands so it can run through the installed
`cefari` CLI.

## Built-In CSS For Custom Titlebars

Cefari injects opt-in drag-region utility classes into trusted main-frame pages.
See [Cefari CSS Contract](../css-contract.md).
