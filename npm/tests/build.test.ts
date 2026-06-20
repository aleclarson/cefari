import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { existsSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { runCefariBuild } from "../src/index.js";
import {
  cefariBuildTargetInfo,
  executableNameForTarget,
  hostCefariBuildTarget,
  withPlatformForTest,
  type CefariBuildTarget,
} from "../src/platform.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

async function projectWithBuildConfig(
  options: { daemon?: boolean; workerNative?: boolean } = { daemon: true },
): Promise<{ root: string; runtime: string; cefResources: string }> {
  const root = await mkdtemp(join(tmpdir(), "cefari-build-"));
  const runtime = join(root, "cefari-desktop");
  const cefResources = join(root, "cef-fixture");
  await mkdir(join(root, "ui"), { recursive: true });
  if (options.daemon !== false) {
    await mkdir(join(root, "daemon"), { recursive: true });
    await writeFile(join(root, "daemon/main.ts"), "console.log('daemon');\n");
  }
  await mkdir(join(root, "workers"), { recursive: true });
  await mkdir(cefResources, { recursive: true });
  await writeFile(join(root, "deno.json"), JSON.stringify({ imports: { "cefari/worker": "./.cefari/worker.ts" } }));
  await writeFile(join(root, "ui/deno.json"), JSON.stringify({ imports: { "local/app": "../src/app.ts" } }));
  await writeFile(join(root, "workers/thumbnailer.ts"), "console.log('thumbnailer');\n");
  if (options.workerNative) {
    await mkdir(join(root, "native/windows-x64"), { recursive: true });
    await writeFile(join(root, "native/windows-x64/thumb.exe"), "windows-tool");
  }
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
    `import { deepLinks, defineConfig } from "${configApi}";

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
  browser: {
    webgpu: true,
  },
  capabilities: [
    deepLinks({ schemes: ["buildapp"] }),
  ],
  ${options.workerNative ? `nativeResources: {
    "thumb-tool": {
      target: "bin/thumb.exe",
      sources: {
        "windows-x64": "native/windows-x64/thumb.exe",
      },
      executable: true,
    },
    "missing-tool": {
      target: "bin/thumb",
      sources: {
        "linux-x64": "native/linux-x64/missing-tool",
      },
      executable: true,
    },
  },` : ""}
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
        ${options.workerNative ? `run: ["$resource/workers/thumbnailer/native/bin/thumb.exe"],` : ""}
      },
      ${options.workerNative ? `native: ["thumb-tool", "missing-tool"],` : ""}
    },
  },
  ${options.daemon === false ? "" : `daemon: {
    entry: "daemon/main.ts",
    ${options.workerNative ? `native: ["thumb-tool"],` : ""}
  },`}
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
  const target = hostCefariBuildTarget();
  const denoTarget = cefariBuildTargetInfo(target).denoTarget;
  const workerExecutable = executableNameForTarget("thumbnailer", target);
  const daemonExecutable = executableNameForTarget("build-app-daemon", target);
  const desktopExecutable = executableNameForTarget("build-app", target);
  const viteConfigs: unknown[] = [];
  const spawned: Array<{ command: string; args: string[] }> = [];

  await withPlatformForTest(
    {
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
    },
    async () => {
      await runCefariBuild({ root, release: true });
    },
  );

  assert.match(
    await readFile(join(root, ".cefari/workers.d.ts"), "utf8"),
    /"thumbnailer": InferCefariWorker<typeof worker_0>/,
  );
  const frontendRoot = realpathSync(join(root, "ui"));
  assert.deepEqual(viteConfigs[0], {
    root: frontendRoot,
    resolve: {
      alias: [
        {
          find: "local/app",
          replacement: join(frontendRoot, "../src/app.ts"),
        },
      ],
    },
    configFile: false,
    build: {
      outDir: join(root, "build/frontend"),
      emptyOutDir: true,
      rollupOptions: {
        input: join(frontendRoot, "index.html"),
      },
    },
  });
  assert.deepEqual(spawned[0], {
    command: "deno",
    args: [
      "compile",
      "--config",
      join(root, "deno.json"),
      "--target",
      denoTarget,
      "--allow-read",
      "--output",
      join(root, "build/workers/thumbnailer", workerExecutable),
      join(root, "workers/thumbnailer.ts"),
    ],
  });
  assert.deepEqual(spawned[1], {
    command: "deno",
    args: [
      "compile",
      "--config",
      join(root, "deno.json"),
      "--target",
      denoTarget,
      "--allow-read",
      "--allow-net",
      "--allow-env=CEFARI_DAEMON,CEFARI_DAEMON_LOG,CEFARI_DAEMON_RESOURCES",
      "--output",
      join(root, "build/daemon", daemonExecutable),
      join(root, "daemon/main.ts"),
    ],
  });
  assert.equal(await readFile(join(root, "build/daemon/main.ts"), "utf8"), "console.log('daemon');\n");
  assert.deepEqual(JSON.parse(await readFile(join(root, "build/config/cefari.json"), "utf8")), {
    app: {
      identifier: "dev.cefari.build",
      display_name: "Build App",
      version: "0.1.0",
    },
    browser: {
      webgpu: true,
    },
    deep_links: {
      schemes: ["buildapp"],
    },
    daemon: {
      enabled: true,
      executable: `daemon/${daemonExecutable}`,
    },
    workers: {
      entries: {
        thumbnailer: {
          target: {
            kind: "executable",
            program: `workers/thumbnailer/${workerExecutable}`,
          },
        },
      },
    },
  });
  assert.equal(await readFile(join(root, "build/workers/thumbnailer", workerExecutable), "utf8"), "daemon-executable");
  assert.equal(await readFile(join(root, "build/desktop", desktopExecutable), "utf8"), "desktop-runtime");
  assert.equal(existsSync(join(root, "build/cef/resources/archive.json")), true);
  assert.equal(existsSync(join(root, "build/cef/resources/libcef.dylib")), true);
  const cefManifest = JSON.parse(await readFile(join(root, "build/cef/manifest.json"), "utf8"));
  assert.equal(cefManifest.target, target);
  assert.equal(cefManifest.target_os, cefariBuildTargetInfo(target).os);
  assert.equal(cefManifest.target_arch, cefariBuildTargetInfo(target).arch);
  assert.equal(cefManifest.sha1, "fixture-sha1");
});

test("builds frontend, desktop, and CEF outputs without daemon artifacts when daemon is omitted", async () => {
  const { root, runtime, cefResources } = await projectWithBuildConfig({ daemon: false });
  const target = hostCefariBuildTarget();
  const workerExecutable = executableNameForTarget("thumbnailer", target);
  const desktopExecutable = executableNameForTarget("build-app", target);
  const spawned: Array<{ command: string; args: string[] }> = [];

  await withPlatformForTest(
    {
      async viteBuild() {
        await mkdir(join(root, "build/frontend"), { recursive: true });
        await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
      },
      spawnSync(command, args) {
        spawned.push({ command, args });
        const outputIndex = args.indexOf("--output");
        if (outputIndex !== -1) {
          const output = args[outputIndex + 1];
          if (typeof output === "string") {
            writeFileSync(output, "worker-executable");
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
    },
    async () => {
      await runCefariBuild({ root, release: true });
    },
  );

  assert.deepEqual(spawned, [
    {
      command: "deno",
      args: [
        "compile",
        "--config",
        join(root, "deno.json"),
        "--target",
        cefariBuildTargetInfo(target).denoTarget,
        "--allow-read",
        "--output",
        join(root, "build/workers/thumbnailer", workerExecutable),
        join(root, "workers/thumbnailer.ts"),
      ],
    },
  ]);
  assert.deepEqual(JSON.parse(await readFile(join(root, "build/config/cefari.json"), "utf8")).daemon, {
    enabled: false,
  });
  assert.equal(existsSync(join(root, "build/daemon")), false);
  assert.equal(await readFile(join(root, "build/desktop", desktopExecutable), "utf8"), "desktop-runtime");
  assert.equal(existsSync(join(root, "build/cef/resources/archive.json")), true);
});

test("builds Windows target executables and metadata with target-specific runtime", async () => {
  const { root, cefResources } = await projectWithBuildConfig();
  const target: CefariBuildTarget = "windows-x64";
  const runtime = join(root, "cefari-desktop-windows.exe");
  await writeFile(runtime, "windows-runtime");
  const spawned: Array<{ command: string; args: string[] }> = [];

  await withPlatformForTest(
    {
      async viteBuild() {
        await mkdir(join(root, "build/frontend"), { recursive: true });
        await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
      },
      spawnSync(command, args) {
        spawned.push({ command, args });
        const outputIndex = args.indexOf("--output");
        if (outputIndex !== -1) {
          const output = args[outputIndex + 1];
          if (typeof output === "string") {
            writeFileSync(output, "compiled-executable");
          }
        }
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME_windows_x64: runtime,
        CEFARI_CEF_RESOURCES_DIR: cefResources,
      },
      stdout: {
        write() {
          return true;
        },
      },
    },
    async () => {
      await runCefariBuild({ root, target });
    },
  );

  assert.deepEqual(spawned[0], {
    command: "deno",
    args: [
      "compile",
      "--config",
      join(root, "deno.json"),
      "--target",
      "x86_64-pc-windows-msvc",
      "--allow-read",
      "--output",
      join(root, "build/workers/thumbnailer/thumbnailer.exe"),
      join(root, "workers/thumbnailer.ts"),
    ],
  });
  assert.deepEqual(spawned[1], {
    command: "deno",
    args: [
      "compile",
      "--config",
      join(root, "deno.json"),
      "--target",
      "x86_64-pc-windows-msvc",
      "--allow-read",
      "--allow-net",
      "--allow-env=CEFARI_DAEMON,CEFARI_DAEMON_LOG,CEFARI_DAEMON_RESOURCES",
      "--output",
      join(root, "build/daemon/build-app-daemon.exe"),
      join(root, "daemon/main.ts"),
    ],
  });
  assert.equal(await readFile(join(root, "build/desktop/build-app.exe"), "utf8"), "windows-runtime");
  const config = JSON.parse(await readFile(join(root, "build/config/cefari.json"), "utf8"));
  assert.equal(config.daemon.executable, "daemon/build-app-daemon.exe");
  assert.equal(config.workers.entries.thumbnailer.target.program, "workers/thumbnailer/thumbnailer.exe");
  const cefManifest = JSON.parse(await readFile(join(root, "build/cef/manifest.json"), "utf8"));
  assert.equal(cefManifest.target, "windows-x64");
  assert.equal(cefManifest.target_os, "windows");
  assert.equal(cefManifest.target_arch, "x64");
});

test("copies worker native resources for the requested build target", async () => {
  const { root, cefResources } = await projectWithBuildConfig({ workerNative: true });
  const target: CefariBuildTarget = "windows-x64";
  const runtime = join(root, "cefari-desktop-windows.exe");
  await writeFile(runtime, "windows-runtime");

  await withPlatformForTest(
    {
      async viteBuild() {
        await mkdir(join(root, "build/frontend"), { recursive: true });
        await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
      },
      spawnSync(command, args) {
        const outputIndex = args.indexOf("--output");
        if (command === "deno" && outputIndex !== -1) {
          const output = args[outputIndex + 1];
          if (typeof output === "string") {
            writeFileSync(output, "compiled-executable");
          }
        }
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME_windows_x64: runtime,
        CEFARI_CEF_RESOURCES_DIR: cefResources,
      },
      stdout: {
        write() {
          return true;
        },
      },
    },
    async () => {
      await runCefariBuild({ root, target });
    },
  );

  const nativePayload = join(root, "build/workers/thumbnailer/native/bin/thumb.exe");
  assert.equal(await readFile(nativePayload, "utf8"), "windows-tool");
  assert.notEqual((await stat(nativePayload)).mode & 0o111, 0);
  const daemonNativeResource = join(root, "build/daemon/native/bin/thumb.exe");
  assert.equal(await readFile(daemonNativeResource, "utf8"), "windows-tool");
  assert.notEqual((await stat(daemonNativeResource)).mode & 0o111, 0);
  assert.equal(existsSync(join(root, "build/workers/thumbnailer/native/bin/thumb")), false);
  const config = JSON.parse(await readFile(join(root, "build/config/cefari.json"), "utf8"));
  assert.deepEqual(config.daemon.native, [
    {
      id: "thumb-tool",
      target: "bin/thumb.exe",
      path: "daemon/native/bin/thumb.exe",
      executable: true,
    },
  ]);
  assert.deepEqual(config.workers.entries.thumbnailer.native, [
    {
      id: "thumb-tool",
      target: "bin/thumb.exe",
      path: "workers/thumbnailer/native/bin/thumb.exe",
      executable: true,
    },
  ]);
});

test("build rejects missing selected worker native resources", async () => {
  const { root, cefResources } = await projectWithBuildConfig({ workerNative: true });
  const runtime = join(root, "cefari-desktop-linux");
  await writeFile(runtime, "linux-runtime");

  await withPlatformForTest(
    {
      async viteBuild() {
        await mkdir(join(root, "build/frontend"), { recursive: true });
        await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
      },
      spawnSync(command, args) {
        const outputIndex = args.indexOf("--output");
        if (command === "deno" && outputIndex !== -1) {
          const output = args[outputIndex + 1];
          if (typeof output === "string") {
            writeFileSync(output, "compiled-executable");
          }
        }
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME_linux_x64: runtime,
        CEFARI_CEF_RESOURCES_DIR: cefResources,
      },
      stdout: {
        write() {
          return true;
        },
      },
    },
    async () => {
      await assert.rejects(
        runCefariBuild({ root, target: "linux-x64" }),
        /worker native resource "missing-tool" does not exist: .*native\/linux-x64\/missing-tool/,
      );
    },
  );
});

test("non-host build target requires a target-specific desktop runtime", async () => {
  const { root, cefResources } = await projectWithBuildConfig({ daemon: false });
  const target: CefariBuildTarget = hostCefariBuildTarget() === "windows-x64" ? "linux-x64" : "windows-x64";

  await withPlatformForTest(
    {
      async viteBuild() {
        await mkdir(join(root, "build/frontend"), { recursive: true });
        await writeFile(join(root, "build/frontend/index.html"), "<!doctype html>");
      },
      spawnSync(command, args) {
        const outputIndex = args.indexOf("--output");
        if (command === "deno" && outputIndex !== -1) {
          const output = args[outputIndex + 1];
          if (typeof output === "string") {
            writeFileSync(output, "worker-executable");
          }
        }
        return { status: 0 };
      },
      env: {
        CEFARI_CEF_RESOURCES_DIR: cefResources,
      },
      stdout: {
        write() {
          return true;
        },
      },
    },
    async () => {
      await assert.rejects(
        runCefariBuild({ root, target }),
        new RegExp(`cefari build --target ${target} requires a cefari-desktop runtime for ${target}`),
      );
    },
  );
});
