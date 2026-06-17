import { cp, mkdir, readFile, rm, writeFile, copyFile } from "node:fs/promises";
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import type { SpawnOptions } from "node:child_process";
import type { InlineConfig } from "vite";
import { loadCefariConfig } from "./config.js";
import type { ResolvedCefariConfig } from "./config.js";
import { resolveDesktopRuntime } from "./dev.js";

const CEF_VERSION = "148.4.0";
const CEF_ARCHIVE_VERSION = "148.0.10";

export interface BuildOptions {
  root?: string;
  release?: boolean;
}

export interface BuildDependencies {
  viteBuild(config: InlineConfig): Promise<unknown>;
  spawnSync(command: string, args: string[], options: SpawnOptions): { status: number | null; error?: Error };
  env: NodeJS.ProcessEnv;
  stdout: Pick<NodeJS.WriteStream, "write">;
}

export async function runCefariBuild(options: BuildOptions = {}, deps = defaultBuildDependencies()): Promise<void> {
  const root = resolve(options.root ?? process.cwd());
  const config = await loadCefariConfig({
    root,
    command: "build",
    mode: "production",
  });
  const buildDir = join(root, "build");
  const frontendOut = join(buildDir, "frontend");
  const daemonOut = join(buildDir, "daemon");
  const desktopOut = join(buildDir, "desktop");
  const configOut = join(buildDir, "config");

  await mkdir(frontendOut, { recursive: true });
  await mkdir(daemonOut, { recursive: true });
  await mkdir(desktopOut, { recursive: true });
  await mkdir(configOut, { recursive: true });

  await deps.viteBuild(createViteBuildConfig(config, frontendOut));
  await writeDesktopConfig(config, configOut);
  await buildDaemon(config, daemonOut, deps);
  await prepareCefResources(root, deps);
  await buildDesktop(config, desktopOut, Boolean(options.release), deps);

  deps.stdout.write(`built Cefari project at ${root}\n`);
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

async function writeDesktopConfig(config: ResolvedCefariConfig, outputDir: string): Promise<void> {
  const deepLinkSchemes = config.capabilities
    .filter((capability) => capability.type === "deepLinks")
    .flatMap((capability) => capability.schemes);
  await writeFile(
    join(outputDir, "cefari.json"),
    `${JSON.stringify(
      {
        app: {
          identifier: config.app.identifier,
          display_name: config.app.name,
          version: config.package.version,
        },
        deep_links: {
          schemes: deepLinkSchemes,
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function buildDaemon(config: ResolvedCefariConfig, outputDir: string, deps: BuildDependencies): Promise<void> {
  const source = resolve(config.root, config.daemon.entry);
  const sourceCopy = join(outputDir, "main.ts");
  await copyFile(source, sourceCopy);

  const executable = join(outputDir, daemonExecutableName(config));
  const status = deps.spawnSync("deno", ["compile", "--allow-read", "--allow-net", "--output", executable, source], {
    cwd: config.root,
    stdio: "inherit",
    env: deps.env,
  });
  if (status.error !== undefined) {
    throw status.error;
  }
  if (status.status !== 0) {
    throw new Error(`deno compile failed with status ${status.status}`);
  }
}

async function buildDesktop(
  config: ResolvedCefariConfig,
  outputDir: string,
  release: boolean,
  deps: BuildDependencies,
): Promise<void> {
  const source = resolveDesktopRuntime(config.root, deps, release);
  await copyFile(source, join(outputDir, desktopExecutableName(config)));
}

async function prepareCefResources(root: string, deps: BuildDependencies): Promise<void> {
  const cefDir = join(root, "build", "cef");
  const resourcesDir = join(cefDir, "resources");
  const cacheDir = join(root, "build", "cef-cache");
  const override = deps.env.CEFARI_CEF_RESOURCES_DIR;

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

function defaultBuildDependencies(): BuildDependencies {
  return {
    async viteBuild(config) {
      const { build } = await import("vite");
      return build(config);
    },
    spawnSync: (command, args, options) => spawnSync(command, args, options),
    env: process.env,
    stdout: process.stdout,
  };
}
