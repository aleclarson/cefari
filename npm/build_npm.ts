import { build, emptyDir } from "jsr:@deno/dnt@^0.42.3";

await emptyDir("./dist/npm");

await build({
  entryPoints: [
    { kind: "export", name: ".", path: "./src/index.ts" },
    { kind: "export", name: "./app", path: "./src/app/mod.ts" },
    { kind: "export", name: "./ipc", path: "./src/app/ipc.ts" },
    { kind: "export", name: "./daemon", path: "./src/daemon.ts" },
    { kind: "export", name: "./logs", path: "./src/logs.ts" },
    { kind: "export", name: "./worker", path: "./src/worker.ts" },
    { kind: "bin", name: "cefari", path: "./bin/cefari.ts" },
  ],
  outDir: "./dist/npm",
  shims: {
    deno: true,
  },
  test: false,
  typeCheck: false,
  scriptModule: false,
  mappings: {
    "npm:cmd-ts@^0.15.0": {
      name: "cmd-ts",
      version: "^0.15.0",
    },
    "npm:vite@^8.0.16": {
      name: "vite",
      version: "^8.0.16",
    },
  },
  package: {
    name: "cefari",
    version: "0.1.0",
    description: "Cefari Deno-first developer CLI",
    license: "MIT OR Apache-2.0",
    type: "module",
    engines: {
      node: ">=20.19",
    },
    dependencies: {
      "cmd-ts": "^0.15.0",
      "vite": "^8.0.16",
    },
  },
  postBuild() {
    Deno.copyFileSync("README.md", "dist/npm/README.md");

    const packageJsonPath = "dist/npm/package.json";
    const packageJson = JSON.parse(Deno.readTextFileSync(packageJsonPath));
    packageJson.types = "./esm/src/index.d.ts";
    packageJson.exports = {
      ".": {
        types: "./esm/src/index.d.ts",
        import: "./esm/src/index.js",
      },
      "./app": {
        types: "./esm/src/app/mod.d.ts",
        import: "./esm/src/app/mod.js",
      },
      "./ipc": {
        types: "./esm/src/app/ipc.d.ts",
        import: "./esm/src/app/ipc.js",
      },
      "./daemon": {
        types: "./esm/src/daemon.d.ts",
        import: "./esm/src/daemon.js",
      },
      "./logs": {
        types: "./esm/src/logs.d.ts",
        import: "./esm/src/logs.js",
      },
      "./worker": {
        types: "./esm/src/worker.d.ts",
        import: "./esm/src/worker.js",
      },
    };
    Deno.writeTextFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
  },
});
