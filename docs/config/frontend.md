# `vite`

The `vite` section tells Cefari how to run and build the Vite app.

```ts
vite: {
  root: "frontend",
  configFile: "frontend/vite.config.ts",
  devPort: 5173,
}
```

## Fields

| Field | Required | Description |
| --- | --- | --- |
| `root` | No | Vite project root. Defaults to `frontend`. |
| `configFile` | No | Vite config file path, or `false` to disable Vite config discovery. |
| `devPort` | No | Fixed Vite dev server port. Defaults to `5173`. |

`cefari dev` calls Vite's `createServer` API directly. `cefari build` calls
Vite's `build` API directly and forces output into `build/frontend/`.
