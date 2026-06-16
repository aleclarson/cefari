# Scaffold An App

Use the checked-in Vite React template as the current starting point while the
project creation flow is being redesigned:

```bash
cp -R templates/vite-react-basic my-cefari-app
```

## Template Files

The template includes:

- `cefari.config.ts`
- `frontend/index.html` and React source files
- `daemon/main.ts`
- `README.md`

## Project Name Rules

`app.projectName` is the stable machine name for generated executables. It
must be lowercase and contain only `a-z`, `0-9`, and `-`.

Cefari uses that value for build outputs:

- desktop executable: `<projectName>` or `<projectName>.exe`
- daemon executable: `<projectName>-daemon` or `<projectName>-daemon.exe`

## Project Manifest Shape

A minimal project config looks like this:

```ts
import { defineConfig } from "cefari";

export default defineConfig({
  app: {
    projectName: "my-cefari-app",
    name: "My Cefari App",
    identifier: "dev.cefari.my-cefari-app",
  },
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

Cefari owns the Vite dev and build lifecycle directly. See
[Develop Locally](development.md) for Vite.

For the complete app-developer config reference, see
[`cefari.config.ts` Reference](../config/index.md).

## Agent Skill

Copy the Cefari skill into `.agents/skills/cefari/` when you want project-local
agent guidance. That skill is a signpost to task-oriented Cefari reference
documents for agents working inside apps.
