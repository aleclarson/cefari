import { spawn, spawnSync } from "node:child_process";
import type { SpawnOptions } from "node:child_process";
import type { InlineConfig, ViteDevServer } from "vite";

export type CefariBuildTarget =
  | "darwin-arm64"
  | "darwin-x64"
  | "linux-x64"
  | "linux-arm64"
  | "windows-x64"
  | "windows-arm64";

export type CefariTargetOs = "darwin" | "linux" | "windows";
export type CefariTargetArch = "arm64" | "x64";

export interface CefariBuildTargetInfo {
  target: CefariBuildTarget;
  os: CefariTargetOs;
  arch: CefariTargetArch;
  denoTarget: string;
  executableSuffix: "" | ".exe";
}

export interface Platform {
  createViteServer(config: InlineConfig): Promise<ViteServerLike>;
  env: NodeJS.ProcessEnv;
  process: ProcessSignalHooks;
  spawn(command: string, args: string[], options: SpawnOptions): ChildLike;
  spawnSync(command: string, args: string[], options: SpawnOptions): { status: number | null; error?: Error };
  stdout: Pick<NodeJS.WriteStream, "write">;
  viteBuild(config: InlineConfig): Promise<unknown>;
}

export interface ChildLike {
  once(event: "exit", listener: (code: number | null, signal: NodeJS.Signals | null) => void): unknown;
  kill(signal?: NodeJS.Signals): unknown;
}

export interface ViteServerLike {
  resolvedUrls?: ViteDevServer["resolvedUrls"];
  listen(): Promise<unknown>;
  close(): Promise<unknown>;
}

export interface ProcessSignalHooks {
  once(event: "SIGINT" | "SIGTERM", listener: () => void): unknown;
  off(event: "SIGINT" | "SIGTERM", listener: () => void): unknown;
}

let platform = defaultPlatform();

export function currentPlatform(): Platform {
  return platform;
}

export async function withPlatformForTest<T>(
  override: Partial<Platform>,
  fn: () => T | Promise<T>,
): Promise<Awaited<T>> {
  const previous = platform;
  platform = { ...previous, ...override };
  try {
    return await fn();
  } finally {
    platform = previous;
  }
}

const buildTargets: Record<CefariBuildTarget, CefariBuildTargetInfo> = {
  "darwin-arm64": {
    target: "darwin-arm64",
    os: "darwin",
    arch: "arm64",
    denoTarget: "aarch64-apple-darwin",
    executableSuffix: "",
  },
  "darwin-x64": {
    target: "darwin-x64",
    os: "darwin",
    arch: "x64",
    denoTarget: "x86_64-apple-darwin",
    executableSuffix: "",
  },
  "linux-x64": {
    target: "linux-x64",
    os: "linux",
    arch: "x64",
    denoTarget: "x86_64-unknown-linux-gnu",
    executableSuffix: "",
  },
  "linux-arm64": {
    target: "linux-arm64",
    os: "linux",
    arch: "arm64",
    denoTarget: "aarch64-unknown-linux-gnu",
    executableSuffix: "",
  },
  "windows-x64": {
    target: "windows-x64",
    os: "windows",
    arch: "x64",
    denoTarget: "x86_64-pc-windows-msvc",
    executableSuffix: ".exe",
  },
  "windows-arm64": {
    target: "windows-arm64",
    os: "windows",
    arch: "arm64",
    denoTarget: "aarch64-pc-windows-msvc",
    executableSuffix: ".exe",
  },
};

export function parseCefariBuildTarget(value: string): CefariBuildTarget {
  if (isCefariBuildTarget(value)) {
    return value;
  }
  throw new Error(
    `build target must be one of ${Object.keys(buildTargets).join(", ")}`,
  );
}

export function isCefariBuildTarget(value: string): value is CefariBuildTarget {
  return Object.hasOwn(buildTargets, value);
}

export function cefariBuildTargetInfo(
  target: CefariBuildTarget,
): CefariBuildTargetInfo {
  return buildTargets[target];
}

export function hostCefariBuildTarget(): CefariBuildTarget {
  const os = process.platform === "win32" ? "windows" : process.platform;
  if (
    (os !== "darwin" && os !== "linux" && os !== "windows") ||
    (process.arch !== "x64" && process.arch !== "arm64")
  ) {
    throw new Error(
      `unsupported host build target: ${process.platform}-${process.arch}`,
    );
  }
  return parseCefariBuildTarget(`${os}-${process.arch}`);
}

export function executableNameForTarget(
  stem: string,
  target: CefariBuildTarget,
): string {
  return `${stem}${cefariBuildTargetInfo(target).executableSuffix}`;
}

function defaultPlatform(): Platform {
  return {
    async createViteServer(config) {
      const { createServer } = await import("vite");
      return createServer(config);
    },
    env: process.env,
    process: {
      once: (event, listener) => process.once(event, listener),
      off: (event, listener) => process.off(event, listener),
    },
    spawn: (command, args, options) => spawn(command, args, options),
    spawnSync: (command, args, options) => spawnSync(command, args, options),
    stdout: process.stdout,
    async viteBuild(config) {
      const { build } = await import("vite");
      return build(config);
    },
  };
}
