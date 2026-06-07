# Develop Locally

Run a project in development:

```bash
cargo run -p cefari-cli -- dev PATH
```

If `PATH` is omitted, Cefari uses the current directory.

## What Dev Mode Starts

`cefari dev` starts:

- a frontend dev server
- the Deno daemon entry with watch mode
- the Rust desktop app

When one child process exits or fails, Cefari stops the remaining processes.

## Frontend Dev Server

Without a configured frontend dev command, Cefari serves `frontend/index.html` with a built-in local static server.

Projects can configure a tool-managed frontend dev server:

```toml
[frontend]
dist = "frontend/dist"
dev_command = ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"]
dev_port = 5173
```

`{port}` is replaced with the selected frontend port. Override it from the CLI:

```bash
cargo run -p cefari-cli -- dev PATH --frontend-port 5273
```

Use `--frontend-port 0` only with the built-in static server, where Cefari can bind an available local port itself.

## Vite React Template

The checked-in Vite React template lives at `templates/vite-react-basic/`.

Install workspace dependencies:

```bash
deno install --config templates/vite-react-basic/deno.json
```

Run it with the local Cefari build:

```bash
cargo run -p cefari-cli -- dev templates/vite-react-basic
```

The template is a Deno workspace with `frontend/` and `daemon/` members. Its Cefari manifest uses `deno task` commands so the template can run from the repository-local Cefari build.

## Built-In CSS For Custom Titlebars

Cefari injects opt-in drag-region utility classes into trusted main-frame pages. See [Cefari CSS Contract](../css-contract.md).
