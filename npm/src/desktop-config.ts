import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { ResolvedCefariConfig, WorkerConfigInput } from "./config.js";

export const CEFARI_CONFIG_FILE_ENV = "CEFARI_CONFIG_FILE";

export interface DesktopConfigOptions {
  daemon?: {
    executable: string;
  };
  workers?: Record<string, WorkerConfigInput>;
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
      entries: sourceWorkerEntries(options.workers ?? config.workers),
    },
  };
}

function sourceWorkerEntries(workers: Record<string, WorkerConfigInput>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(workers).map(([id, worker]) => [
      id,
      {
        target: {
          kind: "denoSource",
          entry: worker.entry,
          permissions: worker.permissions,
        },
      },
    ]),
  );
}
