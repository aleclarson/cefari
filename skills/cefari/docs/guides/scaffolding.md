# Scaffold An App

Use `cefari init` to create a minimal Cefari project:

```bash
cefari init my-cefari-app --name "My Cefari App"
```

If no path is supplied, the CLI creates `./cefari-app`.

## Generated Files

The default scaffold creates:

- `cefari.toml`
- `frontend/index.html`
- `daemon/main.ts`
- `.agents/skills/cefari/SKILL.md`
- `README.md`

The command refuses to overwrite an existing path.

## Project Name Rules

`[app].project_name` is the stable machine name for generated executables. It
must be lowercase and contain only `a-z`, `0-9`, and `-`.

Cefari uses that value for build outputs:

- desktop executable: `<project_name>` or `<project_name>.exe`
- daemon executable: `<project_name>-daemon` or `<project_name>-daemon.exe`

## Project Manifest Shape

A minimal project manifest looks like this:

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
```

Add `frontend.build_command` and `frontend.dev_command` when a framework owns
frontend builds or dev serving. See [Develop Locally](development.md) for Vite.

For the complete app-developer manifest reference, see
[`cefari.toml` Reference](../config/index.md).

## Generated Agent Skill

`cefari init` copies the Cefari skill into `.agents/skills/cefari/`. That skill
is a signpost to task-oriented Cefari reference documents for agents working
inside generated apps.
