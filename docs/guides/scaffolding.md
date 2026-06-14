# Scaffold An App

Use `cefari init` to create a minimal Cefari project:

```bash
cefari init my-cefari-app --name "My Cefari App"
```

If no path is supplied, the CLI creates `./cefari-app`.

## Generated Files

The default scaffold creates:

- `cefari.config.ts`
- `frontend/index.html`
- `daemon/main.ts`
- `.agents/skills/cefari/SKILL.md`
- `README.md`

The command refuses to overwrite an existing path.

## Project Name Rules

`app.projectName` is the stable machine name for generated executables. It
must be lowercase and contain only `a-z`, `0-9`, and `-`.

Cefari uses that value for build outputs:

- desktop executable: `<projectName>` or `<projectName>.exe`
- daemon executable: `<projectName>-daemon` or `<projectName>-daemon.exe`

## Project Manifest Shape

A minimal project config looks like this:

```ts
import { defineConfig } from "@cefari/cli";

export default defineConfig({
  app: {
    projectName: "my-cefari-app",
    name: "My Cefari App",
    identifier: "dev.cefari.my-cefari-app",
  },
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

Add `frontend.buildCommand` and `frontend.devCommand` when a framework owns
frontend builds or dev serving. See [Develop Locally](development.md) for Vite.

For the complete app-developer config reference, see
[`cefari.config.ts` Reference](../config/index.md).

## Generated Agent Skill

`cefari init` copies the Cefari skill into `.agents/skills/cefari/`. That skill
is a signpost to task-oriented Cefari reference documents for agents working
inside generated apps.
