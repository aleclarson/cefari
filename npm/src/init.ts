import { mkdir, stat, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

export interface InitOptions {
  path?: string;
  name?: string;
}

const defaultAppIconPng = Buffer.from(
  [
    "iVBORw0KGgoAAAANSUhEUgAAAgAAAAIACAYAAAD0eNT6AAAK3UlEQVR42u3W0QkAIAhFUadouTZ0Qomg5hDPx1nggXijzn0AwCxhBAAQAACAAAAABAAAIAAAAAEAAAgAAEAAAAACAAAQAACAAAAABAAAIAAAAAEAAAgAAEAAAAACAAAQAAAgAAAAAQAACAAAQAAAAAIAABAAAIAAAAAEAAAgAAAAAQAACAAAQAAAAAIAABAAAIAAAAAEAAAgAABAAAAAAgAAEAAAgAAAAAQAACAAAAABAAAIAABAAAAAAgAAEAAAgAAAAAQAACAAAAABAAAIAABAAACAAAAABAAAIAAAAAEAAAgAAEAAAAACAAAQAACAAAAABAAAIAAAAAEAAAgAAEAAAAACAAAQAACAAAAAAWAIABAAAIAAAAAEwERrJwDN+F8CQAAACAAEgAAAEAAIAAEAIAAEAAIAQAAIAAQAgAAQAAIAAAEgAAQAAAJAAAgAAASAABAAAAgAASAAABAAAkAAACAABIAAAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAEAAJAAAAIAASAAAAQAAIAAQAgAAQAAgBAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAQAAIAAEAgAAQAAIAAAEgAAQAAAJAAAgAAASAABAAAAIAASAAAAQAAkAAAAgAAYAAABAAAgABACAABIAAAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAAAYAAEAAAAgABIAAABIAAQAAACAABgAAAEAACQAA4JAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAACAAEgAAAEAAIAAEAIAAEgBEEAIAAEAAIAAABIAAQAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAA4H8JAAEAIAAQAAIAQAAgAAQAgAAQAAgAAAEgABAAAAJAAAgAAASAABAAAAgAASAAABAAAkAAACAABIAAAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAAAQAAgAAQAgABAAAgBAAAgABACAABAACAAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAQAAIAAEAgAAQAAIAAAEgAAQAAAJAAAgAAASAABAAAAgAASAAAAQAAkAAAAgABIAAABAAAgABACAABAACAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAAAgABIAAABAACQAAACAABgAAAEAACAAEAIAAEgABwSAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAACAAEgAAAEAAIAAEAIAAEAAIAQAAIAAHgkAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAEAAIAAEAIAAQAAIAQAAIACMIAAABIAAQAAACQAAgAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAwP8SAAIAQAAgAAQAgABAAAgAAAEgABAAAAJAACAAAASAABAAAAgAASAAABAAAkAAACAABIAAAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQAgABAAAgBAACAABACAABAACAAAASAAEAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAQAAIAAEAgAAQAAIAAAEgAAQAAAJAAAgAAASAABAAAAgAASAAABAAAkAAAAgABIAAABAACAABACAABAACAEAACAAEAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAABAACQAAACAAEgAAAEAACAAEAIAAEAAIAQAAIAAQAgAAQAALAIQEIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAQAAIAAEAIAAQAAIAQAAgAAQAgAAQAEYQAAACQAAgAAAEgABAAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgAAASAABAAAAkAACAAABIAAEAAACAABIAAAEAACQAAAIAAEgAAAQAAIAAEAgP8lAAQAgABAAAgAAAGAABAAAAJAACAAAASAAEAAAAgAASAAABAAAkAAACAABIAAAEAACAABAIAAEAACAAABIAAEAAACQAAIAAAEgAAQAAAIAAEgAAAQAAJAAAAgAASAAABAAAgAAQCAABAAAgBAACAABACAAEAACAAAASAAEAAAAkAAIAAABIAAAAAEAAAgAAAAAQAACAAAQAAAAAIAABAAAIAAAAAEAAAgAAAAAQAACAAAQAAAAAIAAAQAACAAAAABAAAIAABAAAAAAgAAEAAAgAAAAAQAACAAAAABAAAIAABAAAAAAgAAEAAAgAAAAAQAAAgAAEAAAAACAAAQAACAAAAABAAAIAAAAAEAAAgAAEAAAAACAAAQAACAAAAABAAAIAAAAAEAAAgAABAARgAAAQAACAAAQAAAAAIAABAAAIAAAAAEAAAgAAAAAQAACAAAQAAAAAIAABAAAIAAAAAEAAAgAAAAAQAAAgAAEAAAgAAAAAQAACAAAIAuPhX+7RoHa7I8AAAAAElFTkSuQmCC",
  ].join(""),
  "base64",
);

export async function runCefariInit(options: InitOptions = {}): Promise<void> {
  const destination = resolve(options.path ?? "cefari-app");
  if (await exists(destination)) {
    throw new Error(`refusing to initialize existing path ${destination}`);
  }

  const projectName = basenameForProject(destination);
  const displayName = options.name ?? titleFromProjectName(projectName);

  await mkdir(resolve(destination, "frontend"), { recursive: true });
  await mkdir(resolve(destination, "daemon"), { recursive: true });
  await mkdir(resolve(destination, "assets"), { recursive: true });
  await writeFile(resolve(destination, "cefari.config.ts"), configTemplate(projectName, displayName));
  await writeFile(resolve(destination, "assets/512x512.png"), defaultAppIconPng);
  await writeFile(resolve(destination, "frontend/index.html"), frontendTemplate(displayName));
  await writeFile(resolve(destination, "frontend/vite.config.js"), viteConfigTemplate());
  await writeFile(resolve(destination, "daemon/main.ts"), daemonTemplate(displayName));
  await writeFile(resolve(destination, "README.md"), readmeTemplate(displayName));
}

async function exists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

function basenameForProject(path: string): string {
  const name = path.split(/[\\/]/).filter(Boolean).at(-1) ?? "cefari-app";
  return name
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "") || "cefari-app";
}

function titleFromProjectName(projectName: string): string {
  return projectName
    .split("-")
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join(" ");
}

function configTemplate(projectName: string, displayName: string): string {
  return `import { defineConfig } from "cefari";

export default defineConfig({
  app: {
    projectName: "${escapeString(projectName)}",
    name: "${escapeString(displayName)}",
    identifier: "dev.cefari.${escapeString(projectName)}",
    icon: "assets/512x512.png",
  },
  vite: {
    root: "frontend",
    configFile: "frontend/vite.config.js",
    devPort: 5173,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "${escapeString(displayName)}",
    version: "0.1.0",
  },
});
`;
}

function frontendTemplate(displayName: string): string {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${escapeHtml(displayName)}</title>
  </head>
  <body>
    <main>
      <h1>${escapeHtml(displayName)}</h1>
    </main>
  </body>
</html>
`;
}

function viteConfigTemplate(): string {
  return `export default {};
`;
}

function daemonTemplate(displayName: string): string {
  return `console.log("${escapeString(displayName)} daemon started");
`;
}

function readmeTemplate(displayName: string): string {
  return `# ${displayName}

Run the app:

\`\`\`bash
cefari dev
\`\`\`

Build and package it:

\`\`\`bash
cefari build
cefari package
\`\`\`
`;
}

function escapeString(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function escapeHtml(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
