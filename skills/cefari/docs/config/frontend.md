# `[frontend]`

The `[frontend]` table tells Cefari how to serve and build the frontend.

```toml
[frontend]
dist = "frontend/dist"
build_command = ["deno", "task", "build:frontend"]
dev_command = ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"]
dev_port = 5173
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `dist` | Yes | Frontend build output directory copied into `build/frontend/`. |
| `build_command` | No | Command array run before copying `dist` during `cefari build`. |
| `dev_command` | No | Command array used as the frontend dev server during `cefari dev`. |
| `dev_port` | No | Frontend dev server port. Defaults to `5173`. |

## `build_command`

Use `build_command` when a frontend framework owns production builds:

```toml
build_command = ["npm", "--prefix", "frontend", "run", "build"]
```

Cefari runs the command from the project root. After it succeeds, `[frontend].dist`
must exist.

When `build_command` is omitted, Cefari copies `frontend/index.html` for the
minimal scaffold workflow.

## `dev_command`

Use `dev_command` when a frontend framework owns local serving:

```toml
dev_command = ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"]
```

Cefari runs the command from the project root and sets
`CEFARI_FRONTEND_PORT` to the selected port. Every `{port}` placeholder in the
command array is replaced with that port.

Configured dev commands require a fixed port. Do not use `--frontend-port 0`
with `dev_command`; port `0` is only supported by Cefari's built-in static
server.

When `dev_command` is omitted, Cefari serves `frontend/index.html` with a local
static server.
