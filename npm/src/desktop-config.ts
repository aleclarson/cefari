import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { ResolvedCefariConfig, WorkerConfigInput } from "./config.js";
import { selectedWorkerNativePayloads } from "./native-payloads.js";
import { hostCefariBuildTarget } from "./platform.js";

export const CEFARI_CONFIG_FILE_ENV = "CEFARI_CONFIG_FILE";

export interface DesktopConfigOptions {
  daemon?: {
    executable: string;
  };
  workers?: Record<string, DesktopWorkerEntryInput>;
}

export type DesktopWorkerEntryInput = WorkerConfigInput | DesktopWorkerRuntimeEntry;

export type DesktopWorkerRuntimeEntry = DesktopWorkerDenoSourceEntry | DesktopWorkerExecutableEntry;

export interface DesktopWorkerDenoSourceEntry {
  target: {
    kind: "denoSource";
    entry: string;
    permissions: WorkerConfigInput["permissions"];
  };
  native?: DesktopWorkerNativePayload[];
}

export interface DesktopWorkerExecutableEntry {
  target: {
    kind: "executable";
    program: string;
  };
  native?: DesktopWorkerNativePayload[];
}

export interface DesktopWorkerNativePayload {
  target: string;
  path: string;
  executable: boolean;
}

export async function writeDesktopConfig(
  config: ResolvedCefariConfig,
  outputDir: string,
  options: DesktopConfigOptions = {},
): Promise<string> {
  await mkdir(outputDir, { recursive: true });
  const path = join(outputDir, "cefari.json");
  await writeFile(path, `${JSON.stringify(desktopConfigJson(config, options), null, 2)}\n`);
  return path;
}

function desktopConfigJson(config: ResolvedCefariConfig, options: DesktopConfigOptions): unknown {
  const deepLinkSchemes = config.capabilities
    .filter((capability) => capability.type === "deepLinks")
    .flatMap((capability) => capability.schemes);
  return {
    app: {
      identifier: config.app.identifier,
      display_name: config.app.name,
      version: config.package.version,
    },
    browser: {
      webgpu: config.browser.webgpu,
    },
    deep_links: {
      schemes: deepLinkSchemes,
    },
    daemon:
      options.daemon === undefined
        ? {
            enabled: false,
          }
        : {
            enabled: true,
            executable: options.daemon.executable,
          },
    workers: {
      entries: workerEntries(config, options.workers),
    },
  };
}

function workerEntries(
  config: ResolvedCefariConfig,
  workers: Record<string, DesktopWorkerEntryInput> = config.workers,
): Record<string, unknown> {
  const sourceNativePayloads = selectedWorkerNativePayloads(config, hostCefariBuildTarget());
  return Object.fromEntries(
    Object.entries(workers).map(([id, worker]) => {
      if ("target" in worker) {
        return [id, worker];
      }
      const native = sourceNativePayloads
        .filter((selected) => selected.worker === id)
        .map((selected) => ({
          target: selected.payload.target,
          path: selected.payload.src,
          executable: selected.payload.executable,
        }));
      return [
        id,
        {
          target: {
            kind: "denoSource",
            entry: worker.entry,
            permissions: worker.permissions,
          },
          ...(native.length === 0 ? {} : { native }),
        },
      ];
    }),
  );
}
