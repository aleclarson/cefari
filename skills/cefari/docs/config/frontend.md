# `frontend`

The `frontend` section tells Cefari how to serve and build the frontend.

```ts
frontend: {
  dist: "frontend/dist",
  buildCommand: ["deno", "task", "build:frontend"],
  devCommand: ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"],
  devPort: 5173,
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `dist` | Yes | Frontend build output directory copied into `build/frontend/`. |
| `buildCommand` | No | Command array run before copying `dist` during `cefari build`. |
| `devCommand` | No | Command array used as the frontend dev server during `cefari dev`. |
| `devPort` | No | Frontend dev server port. Defaults to `5173`. |

## `buildCommand`

Use `buildCommand` when a frontend framework owns production builds:

```ts
buildCommand: ["npm", "--prefix", "frontend", "run", "build"],
```

Cefari runs the command from the project root. After it succeeds,
`frontend.dist` must exist.

When `buildCommand` is omitted, Cefari copies `frontend/index.html` for the
minimal scaffold workflow.

## `devCommand`

Use `devCommand` when a frontend framework owns local serving:

```ts
devCommand: ["deno", "task", "dev:frontend", "--host", "127.0.0.1", "--port", "{port}"],
```

Cefari runs the command from the project root and sets `CEFARI_FRONTEND_PORT` to
the selected port. Every `{port}` placeholder in the command array is replaced
with that port.

Configured dev commands require a fixed port. Do not use `--frontend-port 0`
with `devCommand`; port `0` is only supported by Cefari's built-in static
server.

When `devCommand` is omitted, Cefari serves `frontend/index.html` with a local
static server.
