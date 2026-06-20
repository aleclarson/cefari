import assert from "node:assert/strict";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import {
  generateWorkerRegistryTypes,
  type ResolvedCefariConfig,
  WORKER_TYPES_PATH,
  workerRegistryTypes,
} from "../src/index.js";

test("renders worker registry declarations", () => {
  const outputPath = resolve("/project/.cefari/workers.d.ts");
  const contents = workerRegistryTypes(
    configWithWorkers("/project"),
    outputPath,
  );

  assert.equal(
    contents,
    `import type worker_0 from "../workers/thumbnailer.ts";
import type { InferCefariWorker } from "cefari/worker";

declare module "cefari/app" {
  interface CefariWorkerRegistry {
    "thumbnailer": InferCefariWorker<typeof worker_0>;
  }
}

export {};
`,
  );
});

test("renders an empty worker registry declaration", () => {
  const config = configWithWorkers("/project");
  config.workers = {};

  assert.equal(
    workerRegistryTypes(config, resolve("/project/.cefari/workers.d.ts")),
    `
declare module "cefari/app" {
  interface CefariWorkerRegistry {
  }
}

export {};
`,
  );
});

test("writes worker registry declarations", async () => {
  const root = await mkdtemp(join(tmpdir(), "cefari-workers-"));
  const outputPath = await generateWorkerRegistryTypes(configWithWorkers(root));

  assert.equal(outputPath, join(root, WORKER_TYPES_PATH));
  assert.match(
    await readFile(outputPath, "utf8"),
    /"thumbnailer": InferCefariWorker<typeof worker_0>/,
  );
});

function configWithWorkers(root: string): ResolvedCefariConfig {
  return {
    root,
    configPath: join(root, "cefari.config.ts"),
    app: {
      projectName: "worker-app",
      name: "Worker App",
      identifier: "dev.cefari.worker",
    },
    browser: {
      webgpu: false,
    },
    capabilities: [],
    workers: {
      thumbnailer: {
        entry: "workers/thumbnailer.ts",
        permissions: {
          read: "none",
          write: "none",
          net: "none",
          env: "none",
          run: "none",
        },
      },
    },
    vite: {
      root: "frontend",
      devPort: 5173,
    },
    daemon: {
      entry: "daemon/main.ts",
    },
    targets: {
      desktop: {
        capabilities: [],
        daemon: {
          entry: "daemon/main.ts",
        },
      },
    },
    package: {
      productName: "Worker App",
      version: "0.1.0",
    },
  };
}
