import { join } from "node:path";
import type { NativeResource, ResolvedCefariConfig } from "./config.js";
import type { CefariBuildTarget } from "./platform.js";

export type NativeRuntimeKind = "worker" | "daemon";

export interface SelectedNativeResource {
  kind: NativeRuntimeKind;
  runtime: string;
  id: string;
  resource: NativeResource;
  source: string;
  resourcePath: string;
}

export function selectedWorkerNativeResources(
  config: ResolvedCefariConfig,
  target: CefariBuildTarget,
): SelectedNativeResource[] {
  return Object.entries(config.workers).flatMap(([worker, workerConfig]) =>
    selectedNativeResources(
      config,
      target,
      "worker",
      worker,
      workerConfig.native,
      (resource) => workerNativeResourcePath(worker, resource),
    ),
  );
}

export function selectedDaemonNativeResources(
  config: ResolvedCefariConfig,
  target: CefariBuildTarget,
): SelectedNativeResource[] {
  if (config.daemon === undefined) {
    return [];
  }
  return selectedNativeResources(
    config,
    target,
    "daemon",
    "daemon",
    config.daemon.native,
    daemonNativeResourcePath,
  );
}

export function selectedNativeResourcesForBuild(
  config: ResolvedCefariConfig,
  target: CefariBuildTarget,
): SelectedNativeResource[] {
  return [
    ...selectedWorkerNativeResources(config, target),
    ...selectedDaemonNativeResources(config, target),
  ];
}

export function nativeResourceBuildPath(buildDir: string, selected: SelectedNativeResource): string {
  return join(buildDir, ...selected.resourcePath.split("/"));
}

export function workerNativeResourcePath(worker: string, resource: NativeResource): string {
  return normalizeResourcePath(["workers", worker, "native", resource.target].join("/"));
}

export function daemonNativeResourcePath(resource: NativeResource): string {
  return normalizeResourcePath(["daemon", "native", resource.target].join("/"));
}

function selectedNativeResources(
  config: ResolvedCefariConfig,
  target: CefariBuildTarget,
  kind: NativeRuntimeKind,
  runtime: string,
  ids: string[],
  resourcePath: (resource: NativeResource) => string,
): SelectedNativeResource[] {
  const selected: SelectedNativeResource[] = [];
  const targets = new Map<string, string>();
  for (const id of ids) {
    const resource = config.nativeResources[id];
    const source = resource.sources[target];
    if (source === undefined) {
      continue;
    }
    const existing = targets.get(resource.target);
    if (existing !== undefined) {
      throw new Error(
        `${kind} ${runtime} native resources "${existing}" and "${id}" both target ${JSON.stringify(resource.target)} for ${target}`,
      );
    }
    targets.set(resource.target, id);
    selected.push({
      kind,
      runtime,
      id,
      resource,
      source,
      resourcePath: resourcePath(resource),
    });
  }
  return selected;
}

function normalizeResourcePath(path: string): string {
  return path.replaceAll("\\", "/");
}
