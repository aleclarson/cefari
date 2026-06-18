import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { startCefariDev } from "../src/index.js";
import type {
  ChildLike,
  DevDependencies,
  ViteServerLike,
} from "../src/index.js";

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
  options: { daemon?: boolean } = { daemon: true },
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
  workers: {
    thumbnailer: {
      entry: "workers/thumbnailer.ts",
      permissions: {
        read: ["$appData/uploads"],
      },
    },
  },
  ${
      options.daemon === false ? "" : `daemon: {
    entry: "daemon/main.ts",
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
  const spawned: Array<
    {
      command: string;
      args: string[];
      options: Parameters<DevDependencies["spawn"]>[2];
      child: FakeChild;
    }
  > = [];
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
  const deps: DevDependencies = {
    async createServer(config) {
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
  };

  const session = await startCefariDev({
    root,
    vitePort: 5555,
    devtoolsPort: 9222,
  }, deps);

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

test("starts Vite and desktop without a daemon when daemon is omitted", async () => {
  const { root, runtime } = await projectWithDevConfig({ daemon: false });
  const spawned: Array<
    {
      command: string;
      args: string[];
      options: Parameters<DevDependencies["spawn"]>[2];
      child: FakeChild;
    }
  > = [];
  let closed = false;
  const deps: DevDependencies = {
    async createServer() {
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
  };

  const session = await startCefariDev({
    root,
    vitePort: 5555,
    devtoolsPort: 9222,
  }, deps);

  assert.equal(session.daemon, undefined);
  assert.equal(spawned.length, 1);
  assert.equal(spawned[0].command, runtime);

  await session.close();

  assert.equal(spawned[0].child.killedWith, "SIGTERM");
  assert.equal(closed, true);
});
