import { mkdir, writeFile } from "node:fs/promises";
import { createServer as createNetServer } from "node:net";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { InlineConfig } from "vite";
import { loadCefariConfig } from "./config.js";
import type { CefariCapability, ResolvedCefariConfig } from "./config.js";
import { CEFARI_CONFIG_FILE_ENV, writeDesktopConfig } from "./desktop-config.js";
import { selectedDaemonNativeResources } from "./native-payloads.js";
import {
  currentPlatform,
  executableNameForTarget,
  hostCefariBuildTarget,
  type CefariBuildTarget,
} from "./platform.js";
import type { ChildLike, ViteServerLike } from "./platform.js";
import { generateWorkerRegistryTypes } from "./workers.js";

export type { ChildLike, ViteServerLike };

const CEFARI_DAEMON_LOG_ENV = "CEFARI_DAEMON_LOG";
const CEFARI_DAEMON_DEV_CWD_ENV = "CEFARI_DAEMON_DEV_CWD";
const CEFARI_DAEMON_DEV_ENTRY_ENV = "CEFARI_DAEMON_DEV_ENTRY";
const CEFARI_DAEMON_RESOURCES_ENV = "CEFARI_DAEMON_RESOURCES";
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

export async function runCefariDev(options: DevOptions = {}): Promise<void> {
  const session = await startCefariDev(options);
  await waitForDevSession(session);
}

export async function startCefariDev(options: DevOptions = {}): Promise<DevSession> {
  const { createViteServer, stdout } = currentPlatform();
  const root = resolve(options.root ?? process.cwd());
  const config = await loadCefariConfig({
    root,
    command: "dev",
    mode: "development",
  });
  const vitePort = options.vitePort ?? config.vite.devPort;
  validateFixedPort(vitePort, "vitePort");
  await generateWorkerRegistryTypes(config);

  const server = await createViteServer(createViteDevConfig(config, vitePort));
  await server.listen();
  const frontendUrl = resolveFrontendUrl(server, vitePort);

  const devtoolsPort = options.devtoolsPort ?? (await availableLocalPort());
  validateFixedPort(devtoolsPort, "devtoolsPort");
  const devtoolsUrl = `http://127.0.0.1:${devtoolsPort}`;
  await writeDevtoolsFile(root, devtoolsPort, devtoolsUrl);
  const desktopConfigFile = await writeDesktopConfig(config, join(root, ".cefari", "config"));

  stdout.write(`frontend dev server: ${frontendUrl}\n`);
  stdout.write(`chrome devtools: ${devtoolsUrl}\n`);
  stdout.write(`chrome-devtools start --browserUrl ${devtoolsUrl}\n`);

  const desktop = spawnDesktop(config, frontendUrl, devtoolsPort, desktopConfigFile);
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
    await waitForDevSession(session);
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
  desktopConfigFile: string,
): ChildLike {
  const { env, spawn } = currentPlatform();
  const runtime = resolveDesktopRuntime(config.root);
  return spawn(runtime, [], {
    cwd: config.root,
    env: {
      ...env,
      CEFARI_FRONTEND_URL: frontendUrl,
      [CEFARI_DEV_MODE_ENV]: "1",
      [CEFARI_DEVTOOLS_PORT_ENV]: devtoolsPort.toString(),
      [CEFARI_CONFIG_FILE_ENV]: desktopConfigFile,
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
  const native = Object.fromEntries(
    selectedDaemonNativeResources(config, hostCefariBuildTarget()).map((selected) => [
      selected.id,
      resolve(config.root, selected.source),
    ]),
  );
  return {
    [CEFARI_DAEMON_DEV_ENTRY_ENV]: config.daemon.entry,
    [CEFARI_DAEMON_DEV_CWD_ENV]: config.root,
    [CEFARI_DAEMON_LOG_ENV]: join(config.root, ".cefari", "daemon.log"),
    [CEFARI_DAEMON_RESOURCES_ENV]: JSON.stringify({
      resourceDir: config.root,
      nativeDir: join(config.root, "native"),
      native,
    }),
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
  release = false,
  target: CefariBuildTarget = hostCefariBuildTarget(),
): string {
  const { env, spawnSync } = currentPlatform();
  const hostTarget = hostCefariBuildTarget();
  const targetRuntimeEnv = desktopRuntimeEnvName(target);
  const binaryName = executableNameForTarget("cefari-desktop", target);
  const targetConfigured = env[targetRuntimeEnv];
  if (targetConfigured !== undefined && targetConfigured !== "") {
    if (!existsSync(targetConfigured)) {
      throw new Error(
        `${targetRuntimeEnv} points to missing cefari-desktop runtime ${targetConfigured}`,
      );
    }
    return targetConfigured;
  }

  if (target !== hostTarget) {
    for (const candidate of bundledTargetRuntimeCandidates(binaryName, target)) {
      if (existsSync(candidate)) {
        return candidate;
      }
    }
    throw new Error(
      `cefari build --target ${target} requires a cefari-desktop runtime for ${target}. ` +
        `Set ${targetRuntimeEnv}=/path/to/cefari-desktop or install a CLI distribution that bundles this target.`,
    );
  }

  const configured = env[CEFARI_DESKTOP_RUNTIME_ENV];
  if (configured !== undefined && configured !== "") {
    if (!existsSync(configured)) {
      throw new Error(
        `${CEFARI_DESKTOP_RUNTIME_ENV} points to missing cefari-desktop runtime ${configured}`,
      );
    }
    return configured;
  }

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
    const status = spawnSync("cargo", args, {
      cwd: dirname(manifest),
      stdio: "inherit",
      env,
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

function desktopRuntimeEnvName(target: CefariBuildTarget): string {
  return `CEFARI_DESKTOP_RUNTIME_${target.replaceAll("-", "_")}`;
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

function bundledTargetRuntimeCandidates(
  binaryName: string,
  target: CefariBuildTarget,
): string[] {
  const moduleDir = dirname(fileURLToPath(import.meta.url));
  const distDir = dirname(moduleDir);
  const exeDir = dirname(process.execPath);
  return [
    join(distDir, "bin", "cefari-runtime", target, binaryName),
    join(distDir, "bin", target, binaryName),
    join(dirname(distDir), "lib", "cefari", target, binaryName),
    join(dirname(distDir), "libexec", "cefari", target, binaryName),
    join(exeDir, "cefari-runtime", target, binaryName),
    join(exeDir, target, binaryName),
    join(dirname(exeDir), "lib", "cefari", target, binaryName),
    join(dirname(exeDir), "libexec", "cefari", target, binaryName),
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

async function waitForDevSession(session: DevSession): Promise<void> {
  const { process: processHooks } = currentPlatform();
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
  processHooks.once("SIGINT", interrupt);
  processHooks.once("SIGTERM", interrupt);

  try {
    const exits = [childExit("cefari desktop app", session.desktop)];
    if (session.daemon !== undefined) {
      exits.push(childExit("deno daemon", session.daemon));
    }
    await Promise.race(exits);
  } finally {
    processHooks.off("SIGINT", interrupt);
    processHooks.off("SIGTERM", interrupt);
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
