import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { runCefariBuild } from "../src/index.js";
import type { BuildDependencies } from "../src/index.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

async function projectWithBuildConfig(): Promise<{ root: string; runtime: string; cefResources: string }> {
  const root = await mkdtemp(join(tmpdir(), "cefari-build-"));
  const runtime = join(root, "cefari-desktop");
  const cefResources = join(root, "cef-fixture");
  await mkdir(join(root, "daemon"), { recursive: true });
  await mkdir(cefResources, { recursive: true });
  await writeFile(join(root, "daemon/main.ts"), "console.log('daemon');\n");
  await writeFile(runtime, "desktop-runtime");
  await writeFile(
    join(cefResources, "archive.json"),
    JSON.stringify({
      name: "cef_binary_148.0.10+fixture.tar.bz2",
      sha1: "fixture-sha1",
    }),
  );
  await writeFile(join(cefResources, "libcef.dylib"), "cef");
  await writeFile(
    join(root, "cefari.config.ts"),
    `import { defineConfig } from "${configApi}";

export default defineConfig({
  app: {
    projectName: "build-app",
    name: "Build App",
    identifier: "dev.cefari.build",
  },
  vite: {
    root: "ui",
    configFile: false,
    devPort: 4444,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Build App",
    version: "0.1.0",
  },
});
`,
  );
  return { root, runtime, cefResources };
}

test("builds frontend, daemon, desktop, and CEF outputs", async () => {
  const { root, runtime, cefResources } = await projectWithBuildConfig();
  const viteConfigs: unknown[] = [];
  const spawned: Array<{ command: string; args: string[] }> = [];
  const deps: BuildDependencies = {
    async viteBuild(config) {
      viteConfigs.push(config);
      await mkdir(join(root, "build/frontend"), { recursive: true });
      await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
    },
    spawnSync(command, args) {
      spawned.push({ command, args });
      const outputIndex = args.indexOf("--output");
      if (outputIndex !== -1) {
        const output = args[outputIndex + 1];
        if (typeof output === "string") {
          // The mock models `deno compile` producing its executable.
          writeFileSync(output, "daemon-executable");
        }
      }
      return { status: 0 };
    },
    env: {
      CEFARI_DESKTOP_RUNTIME: runtime,
      CEFARI_CEF_RESOURCES_DIR: cefResources,
    },
    stdout: {
      write() {
        return true;
      },
    },
  };

  await runCefariBuild({ root, release: true }, deps);

  assert.deepEqual(viteConfigs[0], {
    root: join(root, "ui"),
    configFile: false,
    build: {
      outDir: join(root, "build/frontend"),
      emptyOutDir: true,
      rollupOptions: {
        input: join(root, "ui/index.html"),
      },
    },
  });
  assert.deepEqual(spawned[0], {
    command: "deno",
    args: [
      "compile",
      "--allow-read",
      "--allow-net",
      "--output",
      join(root, "build/daemon/build-app-daemon"),
      join(root, "daemon/main.ts"),
    ],
  });
  assert.equal(await readFile(join(root, "build/daemon/main.ts"), "utf8"), "console.log('daemon');\n");
  assert.equal(await readFile(join(root, "build/desktop/build-app"), "utf8"), "desktop-runtime");
  assert.equal(existsSync(join(root, "build/cef/resources/archive.json")), true);
  assert.equal(existsSync(join(root, "build/cef/resources/libcef.dylib")), true);
  assert.match(await readFile(join(root, "build/cef/manifest.json"), "utf8"), /fixture-sha1/);
});
