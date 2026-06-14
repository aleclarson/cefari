# `cefari.toml` Reference

`cefari.toml` is the app-developer manifest for a Cefari project. The CLI loads
it from the project root for `dev`, `build`, `package`, `clean`, and `info`.

The manifest is strict: unknown fields are rejected.

## Tables

- [`[app]`](app.md): project name, display name, app identifier, and optional
  icons.
- [`[capabilities]`](capabilities.md): opt-in native desktop integrations.
- [`[frontend]`](frontend.md): frontend dist path and optional build/dev
  commands.
- [`[daemon]`](daemon.md): Deno daemon entrypoint.
- [`[package]`](package.md): packaged product name.

## Minimal Manifest

```toml
[app]
project_name = "my-cefari-app"
name = "My Cefari App"
identifier = "dev.cefari.my-cefari-app"

[frontend]
dist = "frontend/dist"
dev_port = 5173

[daemon]
entry = "daemon/main.ts"

[package]
product_name = "My Cefari App"
version = "0.1.0"
```

## Path Rules

Manifest paths are resolved relative to the project directory passed to the CLI.
Generated outputs are written under the project root:

- `build/`
- `dist/`

Keep runtime-maintenance configuration out of `cefari.toml`. This file
describes the app project that Cefari commands operate on.
