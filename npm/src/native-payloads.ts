import { join } from "node:path";
import type { ResolvedCefariConfig, WorkerNativePayload } from "./config.js";
import type { CefariBuildTarget } from "./platform.js";

export interface SelectedWorkerNativePayload {
  worker: string;
  payload: WorkerNativePayload;
  resourcePath: string;
}

export function selectedWorkerNativePayloads(
  config: ResolvedCefariConfig,
  target: CefariBuildTarget,
): SelectedWorkerNativePayload[] {
  return Object.entries(config.workers).flatMap(([worker, workerConfig]) =>
    workerConfig.native
      .filter((payload) => payload.platforms.length === 0 || payload.platforms.includes(target))
      .map((payload) => ({
        worker,
        payload,
        resourcePath: workerNativePayloadResourcePath(worker, payload),
      })),
  );
}

export function workerNativePayloadResourcePath(worker: string, payload: WorkerNativePayload): string {
  return normalizeResourcePath(["workers", worker, "native", payload.target].join("/"));
}

export function workerNativePayloadBuildPath(buildDir: string, selected: SelectedWorkerNativePayload): string {
  return join(buildDir, ...selected.resourcePath.split("/"));
}

function normalizeResourcePath(path: string): string {
  return path.replaceAll("\\", "/");
}
