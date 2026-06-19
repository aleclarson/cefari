import { cp, mkdir, readFile, rm, writeFile, copyFile } from "node:fs/promises";
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { join, resolve } from "node:path";
import type { InlineConfig } from "vite";
import { loadCefariConfig } from "./config.js";
import type { DaemonConfig, ResolvedCefariConfig, WorkerConfig, WorkerPermissionValue } from "./config.js";
import { resolveDesktopRuntime } from "./dev.js";
import type { DesktopWorkerExecutableEntry } from "./desktop-config.js";
import { writeDesktopConfig } from "./desktop-config.js";
import { currentPlatform } from "./platform.js";
import { generateWorkerRegistryTypes } from "./workers.js";

const CEF_VERSION = "148.4.0";
const CEF_ARCHIVE_VERSION = "148.0.10";

export interface BuildOptions {
  root?: string;
  release?: boolean;
}

export async function runCefariBuild(options: BuildOptions = {}): Promise<void> {
  const { stdout, viteBuild } = currentPlatform();
  const root = resolve(options.root ?? process.cwd());
  const config = await loadCefariConfig({
    root,
    command: "build",
    mode: "production",
  });
  const buildDir = join(root, "build");
  const frontendOut = join(buildDir, "frontend");
  const desktopOut = join(buildDir, "desktop");
  const configOut = join(buildDir, "config");
  const workersOut = join(buildDir, "workers");

  await generateWorkerRegistryTypes(config);
  await rm(workersOut, { recursive: true, force: true });
  await mkdir(frontendOut, { recursive: true });
  await mkdir(desktopOut, { recursive: true });
  await mkdir(configOut, { recursive: true });
  if (config.daemon !== undefined) {
    await mkdir(join(buildDir, "daemon"), { recursive: true });
  }
  await mkdir(workersOut, { recursive: true });

  await viteBuild(createViteBuildConfig(config, frontendOut));
  const packagedWorkers = await buildWorkers(config, workersOut);
  await writeDesktopConfig(config, configOut, {
    workers: packagedWorkers,
    daemon:
      config.daemon === undefined
        ? undefined
        : {
            executable: normalizeResourcePath(join("daemon", daemonExecutableName(config))),
          },
  });
  if (config.daemon !== undefined) {
    await buildDaemon(config, config.daemon, join(buildDir, "daemon"));
  }
  await prepareCefResources(root);
  await buildDesktop(config, desktopOut, Boolean(options.release));

  stdout.write(`built Cefari project at ${root}\n`);
}

export function createViteBuildConfig(config: ResolvedCefariConfig, outDir: string): InlineConfig {
  const root = realpathIfExists(resolve(config.root, config.vite.root));
  return {
    root,
    ...viteResolveConfig(root),
    configFile:
      config.vite.configFile === false
        ? false
        : config.vite.configFile === undefined
          ? undefined
          : resolve(config.root, config.vite.configFile),
    build: {
      outDir,
      emptyOutDir: true,
      rollupOptions: {
        input: resolve(root, "index.html"),
      },
    },
  };
}

function realpathIfExists(path: string): string {
  return existsSync(path) ? realpathSync(path) : path;
}

function viteResolveConfig(root: string): Pick<InlineConfig, "resolve"> {
  const alias = denoLocalImportAliases(root);
  return alias.length === 0 ? {} : { resolve: { alias } };
}

function denoLocalImportAliases(root: string): Array<{ find: string; replacement: string }> {
  const configPath = resolve(root, "deno.json");
  if (!existsSync(configPath)) {
    return [];
  }
  const config = JSON.parse(readFileSync(configPath, "utf8")) as { imports?: Record<string, string> };
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

async function buildWorkers(
  config: ResolvedCefariConfig,
  outputDir: string,
): Promise<Record<string, DesktopWorkerExecutableEntry>> {
  const workers: Record<string, DesktopWorkerExecutableEntry> = {};
  for (const [name, worker] of Object.entries(config.workers)) {
    const source = resolve(config.root, worker.entry);
    const destinationDir = join(outputDir, name);
    const executableName = platformExecutableName(name);
    const executable = join(destinationDir, executableName);
    await mkdir(destinationDir, { recursive: true });
    runDenoCompile(
      [
        ...denoConfigArgs(config),
        ...workerPermissionArgs(config.root, worker),
        "--output",
        executable,
        source,
      ],
      config.root,
    );
    workers[name] = {
      target: {
        kind: "executable",
        program: normalizeResourcePath(["workers", name, executableName].join("/")),
      },
    };
  }
  return workers;
}

function workerPermissionArgs(root: string, worker: WorkerConfig): string[] {
  return [
    ...pathPermissionArgs(root, "read", worker.permissions.read),
    ...pathPermissionArgs(root, "write", worker.permissions.write),
    ...namePermissionArgs("net", worker.permissions.net),
    ...namePermissionArgs("env", worker.permissions.env),
    ...pathPermissionArgs(root, "run", worker.permissions.run),
  ];
}

function pathPermissionArgs(root: string, name: string, value: WorkerPermissionValue): string[] {
  if (value === "none") {
    return [];
  }

  const paths = value.map((entry) => compilePermissionPath(root, entry));
  if (paths.some((path) => path === null)) {
    return [`--allow-${name}`];
  }
  return [`--allow-${name}=${paths.join(",")}`];
}

function compilePermissionPath(root: string, value: string): string | null {
  if (value.startsWith("$appData") || value.startsWith("$cache") || value.startsWith("$resource")) {
    return null;
  }
  return resolve(root, value);
}

function namePermissionArgs(name: string, value: WorkerPermissionValue): string[] {
  if (value === "none") {
    return [];
  }
  if (value.length === 0) {
    return [`--allow-${name}`];
  }
  return [`--allow-${name}=${value.join(",")}`];
}

function runDenoCompile(args: string[], cwd: string): void {
  const { env, spawnSync } = currentPlatform();
  const status = spawnSync("deno", ["compile", ...args], {
    cwd,
    stdio: "inherit",
    env,
  });
  if (status.error !== undefined) {
    throw status.error;
  }
  if (status.status !== 0) {
    throw new Error(`deno compile failed with status ${status.status}`);
  }
}

function denoConfigArgs(config: ResolvedCefariConfig): string[] {
  const configPath = resolve(config.root, "deno.json");
  return existsSync(configPath) ? ["--config", configPath] : [];
}

function normalizeResourcePath(path: string): string {
  return path.replaceAll("\\", "/");
}

async function buildDaemon(
  config: ResolvedCefariConfig,
  daemon: DaemonConfig,
  outputDir: string,
): Promise<void> {
  const source = resolve(config.root, daemon.entry);
  const sourceCopy = join(outputDir, "main.ts");
  await copyFile(source, sourceCopy);

  const executable = join(outputDir, daemonExecutableName(config));
  runDenoCompile([...denoConfigArgs(config), "--allow-read", "--allow-net", "--output", executable, source], config.root);
}

async function buildDesktop(
  config: ResolvedCefariConfig,
  outputDir: string,
  release: boolean,
): Promise<void> {
  const source = resolveDesktopRuntime(config.root, release);
  await copyFile(source, join(outputDir, desktopExecutableName(config)));
}

async function prepareCefResources(root: string): Promise<void> {
  const { env } = currentPlatform();
  const cefDir = join(root, "build", "cef");
  const resourcesDir = join(cefDir, "resources");
  const cacheDir = join(root, "build", "cef-cache");
  const override = env.CEFARI_CEF_RESOURCES_DIR;

  if (override !== undefined && override !== "") {
    await rm(resourcesDir, { recursive: true, force: true });
    await cp(override, resourcesDir, { recursive: true });
  }

  if (!existsSync(join(resourcesDir, "archive.json"))) {
    throw new Error(
      `missing CEF resources at ${resourcesDir}; set CEFARI_CEF_RESOURCES_DIR or reuse an existing build/cef/resources cache`,
    );
  }

  const archive = JSON.parse(await readFile(join(resourcesDir, "archive.json"), "utf8")) as {
    name?: string;
    sha1?: string;
  };
  await mkdir(cefDir, { recursive: true });
  await mkdir(cacheDir, { recursive: true });
  await writeFile(
    join(cefDir, "manifest.json"),
    `${JSON.stringify(
      {
        version: CEF_VERSION,
        archive_version: CEF_ARCHIVE_VERSION,
        target_os: process.platform,
        target_arch: process.arch,
        source: archive.name ?? "",
        sha1: archive.sha1 ?? "",
        cache_dir: cacheDir,
        resources_dir: resourcesDir,
      },
      null,
      2,
    )}\n`,
  );
}

function daemonExecutableName(config: ResolvedCefariConfig): string {
  return platformExecutableName(`${config.app.projectName}-daemon`);
}

function desktopExecutableName(config: ResolvedCefariConfig): string {
  return platformExecutableName(config.app.projectName);
}

function platformExecutableName(stem: string): string {
  return process.platform === "win32" ? `${stem}.exe` : stem;
}
