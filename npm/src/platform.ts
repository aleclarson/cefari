import { spawn, spawnSync } from "node:child_process";
import type { SpawnOptions } from "node:child_process";
import type { InlineConfig, ViteDevServer } from "vite";

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
