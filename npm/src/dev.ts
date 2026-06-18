import { mkdir, writeFile } from "node:fs/promises";
import { createServer as createNetServer } from "node:net";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import type { ChildProcess, SpawnOptions } from "node:child_process";
import { fileURLToPath } from "node:url";
import type { InlineConfig, ViteDevServer } from "vite";
import { loadCefariConfig } from "./config.js";
import type { CefariCapability, ResolvedCefariConfig } from "./config.js";
import { generateWorkerRegistryTypes } from "./workers.js";

const CEFARI_DAEMON_LOG_ENV = "CEFARI_DAEMON_LOG";
const CEFARI_DAEMON_DEV_CWD_ENV = "CEFARI_DAEMON_DEV_CWD";
const CEFARI_DAEMON_DEV_ENTRY_ENV = "CEFARI_DAEMON_DEV_ENTRY";
const CEFARI_DESKTOP_RUNTIME_ENV = "CEFARI_DESKTOP_RUNTIME";
const CEFARI_DEV_MODE_ENV = "CEFARI_DEV_MODE";
const CEFARI_DEVTOOLS_PORT_ENV = "CEFARI_DEVTOOLS_PORT";

export interface DevOptions {
  root?: string;
  vitePort?: number;
  devtoolsPort?: number;
  waitForExit?: boolean;
}

export interface DevSession {
  frontendUrl: string;
  devtoolsUrl: string;
  daemon?: ChildLike;
  desktop: ChildLike;
  close(): Promise<void>;
}

export interface ChildLike {
  once(
    event: "exit",
    listener: (code: number | null, signal: NodeJS.Signals | null) => void,
  ): unknown;
  kill(signal?: NodeJS.Signals): unknown;
}

export interface DevDependencies {
  createServer(config: InlineConfig): Promise<ViteServerLike>;
  spawn(command: string, args: string[], options: SpawnOptions): ChildLike;
  spawnSync(
    command: string,
    args: string[],
    options: SpawnOptions,
  ): { status: number | null; error?: Error };
  env: NodeJS.ProcessEnv;
  stdout: Pick<NodeJS.WriteStream, "write">;
  process: {
    once(event: "SIGINT" | "SIGTERM", listener: () => void): unknown;
    off(event: "SIGINT" | "SIGTERM", listener: () => void): unknown;
  };
}

export interface ViteServerLike {
  resolvedUrls?: ViteDevServer["resolvedUrls"];
  listen(): Promise<unknown>;
  close(): Promise<unknown>;
}

export async function runCefariDev(
  options: DevOptions = {},
  deps = defaultDevDependencies(),
): Promise<void> {
  const session = await startCefariDev(options, deps);
  await waitForDevSession(session, deps);
}

export async function startCefariDev(
  options: DevOptions = {},
  deps = defaultDevDependencies(),
): Promise<DevSession> {
  const root = resolve(options.root ?? process.cwd());
  const config = await loadCefariConfig({
    root,
    command: "dev",
    mode: "development",
  });
  const vitePort = options.vitePort ?? config.vite.devPort;
  validateFixedPort(vitePort, "vitePort");
  await generateWorkerRegistryTypes(config);

  const server = await deps.createServer(createViteDevConfig(config, vitePort));
  await server.listen();
  const frontendUrl = resolveFrontendUrl(server, vitePort);

  const devtoolsPort = options.devtoolsPort ?? (await availableLocalPort());
  validateFixedPort(devtoolsPort, "devtoolsPort");
  const devtoolsUrl = `http://127.0.0.1:${devtoolsPort}`;
  await writeDevtoolsFile(root, devtoolsPort, devtoolsUrl);

  deps.stdout.write(`frontend dev server: ${frontendUrl}\n`);
  deps.stdout.write(`chrome devtools: ${devtoolsUrl}\n`);
  deps.stdout.write(`chrome-devtools start --browserUrl ${devtoolsUrl}\n`);

  const desktop = spawnDesktop(config, frontendUrl, devtoolsPort, deps);
  const session = {
    frontendUrl,
    devtoolsUrl,
    daemon: undefined,
    desktop,
    async close() {
      desktop.kill("SIGTERM");
      await server.close();
    },
  };

  if (options.waitForExit) {
    await waitForDevSession(session, deps);
  }

  return session;
}

export function createViteDevConfig(
  config: ResolvedCefariConfig,
  port: number,
): InlineConfig {
  const root = resolve(config.root, config.vite.root);
  return {
    root,
    ...viteResolveConfig(root),
    configFile: config.vite.configFile === false
      ? false
      : config.vite.configFile === undefined
      ? undefined
      : resolve(config.root, config.vite.configFile),
    server: {
      host: "127.0.0.1",
      port,
      strictPort: true,
    },
  };
}

function viteResolveConfig(root: string): Pick<InlineConfig, "resolve"> {
  const alias = denoLocalImportAliases(root);
  return alias.length === 0 ? {} : { resolve: { alias } };
}

function denoLocalImportAliases(
  root: string,
): Array<{ find: string; replacement: string }> {
  const configPath = resolve(root, "deno.json");
  if (!existsSync(configPath)) {
    return [];
  }
  const config = JSON.parse(readFileSync(configPath, "utf8")) as {
    imports?: Record<string, string>;
  };
  return Object.entries(config.imports ?? {})
    .filter(([, value]) => isLocalImportTarget(value))
    .map(([find, value]) => ({
      find,
      replacement: resolve(root, value),
    }));
}

function isLocalImportTarget(value: string): boolean {
  return value.startsWith(".") || value.startsWith("/");
}

function spawnDesktop(
  config: ResolvedCefariConfig,
  frontendUrl: string,
  devtoolsPort: number,
  deps: DevDependencies,
): ChildLike {
  const runtime = resolveDesktopRuntime(config.root, deps);
  return deps.spawn(runtime, [], {
    cwd: config.root,
    env: {
      ...deps.env,
      CEFARI_FRONTEND_URL: frontendUrl,
      [CEFARI_DEV_MODE_ENV]: "1",
      [CEFARI_DEVTOOLS_PORT_ENV]: devtoolsPort.toString(),
      CEFARI_RESOURCE_DIR: config.root,
      ...daemonDevEnv(config),
      ...trayIconEnv(config),
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
}

function daemonDevEnv(config: ResolvedCefariConfig): Record<string, string> {
  if (config.daemon === undefined) {
    return {};
  }
  return {
    [CEFARI_DAEMON_DEV_ENTRY_ENV]: config.daemon.entry,
    [CEFARI_DAEMON_DEV_CWD_ENV]: config.root,
    [CEFARI_DAEMON_LOG_ENV]: join(config.root, ".cefari", "daemon.log"),
  };
}

function trayIconEnv(config: ResolvedCefariConfig): Record<string, string> {
  const tray = config.capabilities.find(
    (capability): capability is Extract<CefariCapability, { type: "tray" }> => {
      return capability.type === "tray";
    },
  );
  return tray === undefined
    ? {}
    : { CEFARI_TRAY_ICON: resolve(config.root, tray.icon) };
}

export function resolveDesktopRuntime(
  root: string,
  deps: Pick<DevDependencies, "env" | "spawnSync">,
  release = false,
): string {
  const configured = deps.env[CEFARI_DESKTOP_RUNTIME_ENV];
  if (configured !== undefined && configured !== "") {
    if (!existsSync(configured)) {
      throw new Error(
        `${CEFARI_DESKTOP_RUNTIME_ENV} points to missing cefari-desktop runtime ${configured}`,
      );
    }
    return configured;
  }

  const binaryName = process.platform === "win32"
    ? "cefari-desktop.exe"
    : "cefari-desktop";
  for (const candidate of bundledRuntimeCandidates(binaryName)) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  const manifest = findWorkspaceManifest(root);
  if (manifest !== null) {
    const args = ["build", "--manifest-path", manifest, "-p", "cefari-desktop"];
    if (release) {
      args.push("--release");
    }
    const status = deps.spawnSync("cargo", args, {
      cwd: dirname(manifest),
      stdio: "inherit",
      env: deps.env,
    });
    if (status.error !== undefined) {
      throw status.error;
    }
    if (status.status !== 0) {
      throw new Error(
        `cargo build -p cefari-desktop failed with status ${status.status}`,
      );
    }
    const runtime = join(
      dirname(manifest),
      "target",
      release ? "release" : "debug",
      binaryName,
    );
    if (existsSync(runtime)) {
      return runtime;
    }
  }

  throw new Error(
    `cefari-desktop runtime was not found; set ${CEFARI_DESKTOP_RUNTIME_ENV} to a prebuilt runtime`,
  );
}

function bundledRuntimeCandidates(binaryName: string): string[] {
  const moduleDir = dirname(fileURLToPath(import.meta.url));
  const distDir = dirname(moduleDir);
  const exeDir = dirname(process.execPath);
  return [
    join(distDir, "bin", binaryName),
    join(distDir, "bin", "cefari-runtime", binaryName),
    join(dirname(distDir), "lib", "cefari", binaryName),
    join(dirname(distDir), "libexec", "cefari", binaryName),
    join(exeDir, binaryName),
    join(exeDir, "cefari-runtime", binaryName),
    join(dirname(exeDir), "lib", "cefari", binaryName),
    join(dirname(exeDir), "libexec", "cefari", binaryName),
  ];
}

function findWorkspaceManifest(start: string): string | null {
  let cursor = resolve(start);
  while (true) {
    const manifest = join(cursor, "Cargo.toml");
    if (existsSync(manifest)) {
      return manifest;
    }
    const parent = dirname(cursor);
    if (parent === cursor) {
      return null;
    }
    cursor = parent;
  }
}

function resolveFrontendUrl(server: ViteServerLike, port: number): string {
  const localUrl = server.resolvedUrls?.local[0];
  return localUrl?.replace(/\/$/, "") ?? `http://127.0.0.1:${port}`;
}

async function writeDevtoolsFile(
  root: string,
  port: number,
  browserUrl: string,
): Promise<void> {
  const devtoolsDir = join(root, ".cefari");
  await mkdir(devtoolsDir, { recursive: true });
  await writeFile(
    join(devtoolsDir, "devtools.json"),
    `${JSON.stringify({ port, browserUrl }, null, 2)}\n`,
  );
}

function validateFixedPort(port: number, name: string): void {
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`${name} must be an integer from 1 to 65535`);
  }
}

async function availableLocalPort(): Promise<number> {
  return await new Promise((resolvePort, reject) => {
    const server = createNetServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close();
      if (address === null || typeof address === "string") {
        reject(new Error("failed to allocate a local DevTools port"));
        return;
      }
      resolvePort(address.port);
    });
  });
}

async function waitForDevSession(
  session: DevSession,
  deps: DevDependencies,
): Promise<void> {
  let shuttingDown = false;
  const shutdown = async () => {
    if (shuttingDown) {
      return;
    }
    shuttingDown = true;
    await session.close();
  };

  const interrupt = () => {
    void shutdown();
  };
  deps.process.once("SIGINT", interrupt);
  deps.process.once("SIGTERM", interrupt);

  try {
    const exits = [childExit("cefari desktop app", session.desktop)];
    if (session.daemon !== undefined) {
      exits.push(childExit("deno daemon", session.daemon));
    }
    await Promise.race(exits);
  } finally {
    deps.process.off("SIGINT", interrupt);
    deps.process.off("SIGTERM", interrupt);
    await shutdown();
  }
}

async function childExit(description: string, child: ChildLike): Promise<void> {
  await new Promise<void>((resolveChild, reject) => {
    child.once("exit", (code, signal) => {
      if (code === 0 || signal !== null) {
        resolveChild();
      } else {
        reject(new Error(`${description} failed with status ${code}`));
      }
    });
  });
}

function defaultDevDependencies(): DevDependencies {
  return {
    async createServer(config) {
      const { createServer } = await import("vite");
      return createServer(config);
    },
    spawn: (command, args, options) =>
      spawn(command, args, options) as ChildProcess,
    spawnSync: (command, args, options) => spawnSync(command, args, options),
    env: process.env,
    stdout: process.stdout,
    process,
  };
}
