import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { SpawnOptions } from "node:child_process";
import test from "node:test";
import { runCefariDev, startCefariDev } from "../src/index.js";
import type { ChildLike, ViteServerLike } from "../src/index.js";
import { hostCefariBuildTarget, withPlatformForTest } from "../src/platform.js";

const testDir = dirname(fileURLToPath(import.meta.url));
const configApi = pathToFileURL(resolve(testDir, "../src/index.js")).href;

class FakeChild extends EventEmitter implements ChildLike {
  killedWith: NodeJS.Signals | undefined;

  kill(signal?: NodeJS.Signals): boolean {
    this.killedWith = signal;
    return true;
  }
}

async function projectWithDevConfig(
  options: { daemon?: boolean; workerNative?: boolean } = { daemon: true },
): Promise<{ root: string; runtime: string }> {
  const root = await mkdtemp(join(tmpdir(), "cefari-dev-"));
  const runtime = join(root, "cefari-desktop");
  await mkdir(join(root, "ui"), { recursive: true });
  await writeFile(runtime, "");
  await writeFile(
    join(root, "ui/deno.json"),
    JSON.stringify({ imports: { "local/app": "../src/app.ts" } }),
  );
  await writeFile(
    join(root, "cefari.config.ts"),
    `import { defineConfig, tray } from "${configApi}";

export default defineConfig({
  app: {
    projectName: "dev-app",
    name: "Dev App",
    identifier: "dev.cefari.dev",
  },
  vite: {
    root: "ui",
    configFile: false,
    devPort: 4444,
  },
  capabilities: [
    tray({ icon: "assets/tray.png" }),
  ],
  ${options.workerNative ? `nativeResources: {
    "thumb-tool": {
      target: "bin/thumb",
      sources: {
        "${hostCefariBuildTarget()}": "native/thumb",
      },
      executable: true,
    },
  },` : ""}
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
      },
      ${options.workerNative ? `native: ["thumb-tool"],` : ""}
    },
  },
  ${
      options.daemon === false ? "" : `daemon: {
    entry: "daemon/main.ts",
    ${options.workerNative ? `native: ["thumb-tool"],` : ""}
  },`
    }
  package: {
    productName: "Dev App",
    version: "0.1.0",
  },
});
`,
  );
  return { root, runtime };
}

test("starts Vite and desktop with daemon stream dev inputs", async () => {
  const { root, runtime } = await projectWithDevConfig();
  const spawned: Array<{ command: string; args: string[]; options: SpawnOptions; child: FakeChild }> = [];
  const viteConfigs: unknown[] = [];
  let closed = false;
  const server: ViteServerLike = {
    resolvedUrls: {
      local: ["http://127.0.0.1:5555/"],
      network: [],
    },
    async listen() {},
    async close() {
      closed = true;
    },
  };
  const session = await withPlatformForTest(
    {
      async createViteServer(config) {
        viteConfigs.push(config);
        return server;
      },
      spawn(command, args, options) {
        const child = new FakeChild();
        spawned.push({ command, args, options, child });
        return child;
      },
      spawnSync() {
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME: runtime,
      },
      stdout: {
        write() {
          return true;
        },
      },
      process: {
        once() {
          return undefined;
        },
        off() {
          return undefined;
        },
      },
    },
    async () => await startCefariDev({ root, vitePort: 5555, devtoolsPort: 9222 }),
  );

  assert.deepEqual(viteConfigs[0], {
    root: join(root, "ui"),
    resolve: {
      alias: [
        {
          find: "local/app",
          replacement: join(root, "src/app.ts"),
        },
      ],
    },
    configFile: false,
    server: {
      host: "127.0.0.1",
      port: 5555,
      strictPort: true,
    },
  });
  assert.equal(session.frontendUrl, "http://127.0.0.1:5555");
  assert.equal(session.devtoolsUrl, "http://127.0.0.1:9222");
  assert.match(
    await readFile(join(root, ".cefari/workers.d.ts"), "utf8"),
    /"thumbnailer": InferCefariWorker<typeof worker_0>/,
  );
  assert.deepEqual(JSON.parse(await readFile(join(root, ".cefari/config/cefari.json"), "utf8")), {
    app: {
      identifier: "dev.cefari.dev",
      display_name: "Dev App",
      version: "0.1.0",
    },
    browser: {
      webgpu: false,
    },
    deep_links: {
      schemes: [],
    },
    logs: {
      local: {
        enabled: true,
      },
      exporters: {
        sentry: {
          enabled: false,
          level: "info",
          sampleRate: 1,
        },
      },
    },
    daemon: {
      enabled: false,
    },
    workers: {
      entries: {
        thumbnailer: {
          target: {
            kind: "denoSource",
            entry: "workers/thumbnailer.ts",
            permissions: {
              read: ["$appData/uploads"],
              write: "none",
              net: "none",
              env: "none",
              run: "none",
              ffi: "none",
            },
          },
        },
      },
    },
  });

  assert.equal(session.daemon, undefined);
  assert.equal(spawned.length, 1);
  assert.equal(spawned[0].command, runtime);
  assert.deepEqual(spawned[0].args, []);
  assert.equal(
    spawned[0].options.env?.CEFARI_FRONTEND_URL,
    "http://127.0.0.1:5555",
  );
  assert.equal(spawned[0].options.env?.CEFARI_DEV_MODE, "1");
  assert.equal(spawned[0].options.env?.CEFARI_DEVTOOLS_PORT, "9222");
  assert.equal(
    spawned[0].options.env?.CEFARI_CONFIG_FILE,
    join(root, ".cefari/config/cefari.json"),
  );
  assert.equal(spawned[0].options.env?.CEFARI_RESOURCE_DIR, root);
  assert.equal(
    spawned[0].options.env?.CEFARI_DAEMON_DEV_ENTRY,
    "daemon/main.ts",
  );
  assert.equal(spawned[0].options.env?.CEFARI_DAEMON_DEV_CWD, root);
  assert.equal(
    spawned[0].options.env?.CEFARI_DAEMON_LOG,
    join(root, ".cefari", "daemon.log"),
  );
  assert.equal(
    spawned[0].options.env?.CEFARI_TRAY_ICON,
    join(root, "assets/tray.png"),
  );

  await session.close();

  assert.equal(spawned[0].child.killedWith, "SIGTERM");
  assert.equal(closed, true);
});

test("writes source worker native resource paths for dev runtime config", async () => {
  const { root, runtime } = await projectWithDevConfig({ daemon: false, workerNative: true });

  await withPlatformForTest(
    {
      async createViteServer() {
        return {
          resolvedUrls: {
            local: ["http://127.0.0.1:5555/"],
            network: [],
          },
          async listen() {},
          async close() {},
        };
      },
      spawn() {
        return new FakeChild();
      },
      spawnSync() {
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME: runtime,
      },
      stdout: {
        write() {
          return true;
        },
      },
      process: {
        once() {
          return undefined;
        },
        off() {
          return undefined;
        },
      },
    },
    async () => {
      const session = await startCefariDev({ root });
      await session.close();
    },
  );

  const config = JSON.parse(await readFile(join(root, ".cefari/config/cefari.json"), "utf8"));
  assert.deepEqual(config.workers.entries.thumbnailer.native, [
    {
      id: "thumb-tool",
      target: "bin/thumb",
      path: "native/thumb",
      executable: true,
    },
  ]);
});

test("passes source daemon native resource paths in dev env", async () => {
  const { root, runtime } = await projectWithDevConfig({ workerNative: true });
  const spawned: Array<{ command: string; args: string[]; options: SpawnOptions; child: FakeChild }> = [];

  await withPlatformForTest(
    {
      async createViteServer() {
        return {
          resolvedUrls: {
            local: ["http://127.0.0.1:5555/"],
            network: [],
          },
          async listen() {},
          async close() {},
        };
      },
      spawn(command, args, options) {
        const child = new FakeChild();
        spawned.push({ command, args, options, child });
        return child;
      },
      spawnSync() {
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME: runtime,
      },
      stdout: {
        write() {
          return true;
        },
      },
      process: {
        once() {
          return undefined;
        },
        off() {
          return undefined;
        },
      },
    },
    async () => {
      const session = await startCefariDev({ root });
      await session.close();
    },
  );

  const env = spawned[0].options.env as NodeJS.ProcessEnv;
  const resources = JSON.parse(env.CEFARI_DAEMON_RESOURCES ?? "{}");
  assert.equal(resources.resourceDir, root);
  assert.equal(resources.native["thumb-tool"], join(root, "native/thumb"));
});

test("starts Vite and desktop without a daemon when daemon is omitted", async () => {
  const { root, runtime } = await projectWithDevConfig({ daemon: false });
  const spawned: Array<{ command: string; args: string[]; options: SpawnOptions; child: FakeChild }> = [];
  let closed = false;

  const session = await withPlatformForTest(
    {
      async createViteServer() {
        return {
          resolvedUrls: {
            local: ["http://127.0.0.1:5555/"],
            network: [],
          },
          async listen() {},
          async close() {
            closed = true;
          },
        };
      },
      spawn(command, args, options) {
        const child = new FakeChild();
        spawned.push({ command, args, options, child });
        return child;
      },
      spawnSync() {
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME: runtime,
      },
      stdout: {
        write() {
          return true;
        },
      },
      process: {
        once() {
          return undefined;
        },
        off() {
          return undefined;
        },
      },
    },
    async () => await startCefariDev({ root, vitePort: 5555, devtoolsPort: 9222 }),
  );

  assert.equal(session.daemon, undefined);
  assert.equal(spawned.length, 1);
  assert.equal(spawned[0].command, runtime);

  await session.close();

  assert.equal(spawned[0].child.killedWith, "SIGTERM");
  assert.equal(closed, true);
});

test("dev wait path subscribes to signals and cleans up spawned processes", async () => {
  const { root, runtime } = await projectWithDevConfig();
  const spawned: FakeChild[] = [];
  const signalEvents: string[] = [];
  let closed = false;
  const server: ViteServerLike = {
    resolvedUrls: {
      local: ["http://127.0.0.1:5555/"],
      network: [],
    },
    async listen() {},
    async close() {
      closed = true;
    },
  };

  await withPlatformForTest(
    {
      async createViteServer() {
        return server;
      },
      spawn() {
        const child = new FakeChild();
        spawned.push(child);
        if (spawned.length === 1) {
          setImmediate(() => child.emit("exit", 0, null));
        }
        return child;
      },
      spawnSync() {
        return { status: 0 };
      },
      env: {
        CEFARI_DESKTOP_RUNTIME: runtime,
      },
      stdout: {
        write() {
          return true;
        },
      },
      process: {
        once(event) {
          signalEvents.push(`once:${event}`);
          return undefined;
        },
        off(event) {
          signalEvents.push(`off:${event}`);
          return undefined;
        },
      },
    },
    async () => await runCefariDev({ root, vitePort: 5555, devtoolsPort: 9222 }),
  );

  assert.deepEqual(signalEvents, ["once:SIGINT", "once:SIGTERM", "off:SIGINT", "off:SIGTERM"]);
  assert.equal(spawned[0].killedWith, "SIGTERM");
  assert.equal(closed, true);
});
