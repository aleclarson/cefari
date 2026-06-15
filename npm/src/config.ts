import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { runnerImport } from "vite";
import type { CliPackageCommand, CliTopLevelCommand } from "./cli.js";

export type ConfigMode = "development" | "production";

export interface CefariConfigContext {
  command: CliTopLevelCommand;
  packageCommand: CliPackageCommand | null;
  mode: ConfigMode;
  root: string;
}

export interface CefariConfigInput {
  app: AppConfigInput;
  capabilities?: CefariCapability[];
  vite?: ViteConfigInput;
  daemon: DaemonConfigInput;
  package: PackageConfigInput;
  frontend?: never;
}

export type CefariConfigExport =
  | CefariConfigInput
  | ((context: CefariConfigContext) => CefariConfigInput | Promise<CefariConfigInput>);

export interface AppConfigInput {
  projectName: string;
  name: string;
  identifier: string;
  icon?: string;
}

export interface ViteConfigInput {
  root?: string;
  configFile?: string | false;
  devPort?: number;
}

export interface DaemonConfigInput {
  entry: string;
}

export interface PackageConfigInput {
  productName: string;
  version: string;
}

export interface TrayCapabilityOptions {
  icon: string;
}

export interface TrayCapabilityInput extends TrayCapabilityOptions {
  type: "tray";
}

export type CefariCapability = TrayCapabilityInput;

export interface AppConfig extends AppConfigInput {}

export interface ViteConfig {
  root: string;
  configFile?: string | false;
  devPort: number;
}

export interface DaemonConfig extends DaemonConfigInput {}

export interface PackageConfig extends PackageConfigInput {}

export interface ResolvedCefariConfig {
  root: string;
  configPath: string;
  app: AppConfig;
  capabilities: CefariCapability[];
  vite: ViteConfig;
  daemon: DaemonConfig;
  package: PackageConfig;
}

export interface SerializableProjectConfig {
  app: AppConfig;
  capabilities: CefariCapability[];
  vite: ViteConfig;
  daemon: DaemonConfig;
  package: PackageConfig;
}

export interface LoadCefariConfigOptions {
  root?: string;
  configFile?: string;
  command?: CliTopLevelCommand;
  packageCommand?: CliPackageCommand | null;
  mode?: ConfigMode;
}

interface CefariConfigModule {
  default?: unknown;
}

export function defineConfig(config: CefariConfigExport): CefariConfigExport {
  return config;
}

export function tray(config: TrayCapabilityOptions): TrayCapabilityInput {
  return { type: "tray", ...config };
}

export async function loadCefariConfig(options: LoadCefariConfigOptions = {}): Promise<ResolvedCefariConfig> {
  const root = resolve(options.root ?? process.cwd());
  const configPath = resolve(root, options.configFile ?? "cefari.config.ts");
  if (!existsSync(configPath)) {
    throw new Error(`Cefari config not found at ${configPath}`);
  }

  const context: CefariConfigContext = {
    command: options.command ?? "unknown",
    packageCommand: options.packageCommand ?? null,
    mode: options.mode ?? "development",
    root,
  };

  const { module } = await runnerImport<CefariConfigModule>(pathToFileURL(configPath).href, {
    root,
    logLevel: "silent",
    resolve: {
      alias: [{ find: /^cefari$/, replacement: resolve(dirname(fileURLToPath(import.meta.url)), "index.js") }],
    },
  });

  if (!Object.hasOwn(module, "default")) {
    throw new Error(`Cefari config at ${configPath} must have a default export`);
  }

  const configExport = module.default;
  const config = typeof configExport === "function" ? await configExport(context) : configExport;
  return normalizeConfig(config, { root, configPath });
}

export function toSerializableProjectConfig(config: ResolvedCefariConfig): SerializableProjectConfig {
  return {
    app: { ...config.app },
    capabilities: config.capabilities.map((capability) => ({ ...capability })),
    vite: { ...config.vite },
    daemon: { ...config.daemon },
    package: { ...config.package },
  };
}

function normalizeConfig(config: unknown, paths: { root: string; configPath: string }): ResolvedCefariConfig {
  const input = asRecord(config, "default export");
  if ("frontend" in input) {
    throw new Error("frontend is no longer supported in cefari.config.ts; use vite instead");
  }

  const app = normalizeApp(input.app);
  const capabilities = normalizeCapabilities(input.capabilities);
  const vite = normalizeVite(input.vite);
  const daemon = normalizeDaemon(input.daemon);
  const packageConfig = normalizePackage(input.package);

  return {
    root: paths.root,
    configPath: paths.configPath,
    app,
    capabilities,
    vite,
    daemon,
    package: packageConfig,
  };
}

function normalizeApp(value: unknown): AppConfig {
  const app = asRecord(value, "app");
  const projectName = requiredString(app.projectName, "app.projectName");
  if (!/^[a-z0-9-]+$/.test(projectName)) {
    throw new Error("app.projectName must match ^[a-z0-9-]+$");
  }
  return {
    projectName,
    name: requiredString(app.name, "app.name"),
    identifier: requiredString(app.identifier, "app.identifier"),
    ...(app.icon === undefined ? {} : { icon: relativePath(app.icon, "app.icon") }),
  };
}

function normalizeCapabilities(value: unknown): CefariCapability[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error("capabilities must be an array");
  }

  let trayCount = 0;
  return value.map((entry, index) => {
    const capability = asRecord(entry, `capabilities[${index}]`);
    const type = requiredString(capability.type, `capabilities[${index}].type`);
    if (type !== "tray") {
      throw new Error(`capabilities[${index}].type must be "tray"`);
    }
    trayCount += 1;
    if (trayCount > 1) {
      throw new Error("capabilities must not include more than one tray capability");
    }
    return {
      type: "tray",
      icon: relativePath(capability.icon, `capabilities[${index}].icon`),
    };
  });
}

function normalizeVite(value: unknown): ViteConfig {
  if (value === undefined) {
    return {
      root: "frontend",
      devPort: 5173,
    };
  }

  const vite = asRecord(value, "vite");
  const configFile = vite.configFile === undefined ? undefined : normalizeViteConfigFile(vite.configFile);
  return {
    root: vite.root === undefined ? "frontend" : relativePath(vite.root, "vite.root"),
    ...(configFile === undefined ? {} : { configFile }),
    devPort: vite.devPort === undefined ? 5173 : port(vite.devPort, "vite.devPort"),
  };
}

function normalizeViteConfigFile(value: unknown): string | false {
  if (value === false) {
    return false;
  }
  return relativePath(value, "vite.configFile");
}

function normalizeDaemon(value: unknown): DaemonConfig {
  const daemon = asRecord(value, "daemon");
  return {
    entry: relativePath(daemon.entry, "daemon.entry"),
  };
}

function normalizePackage(value: unknown): PackageConfig {
  const packageConfig = asRecord(value, "package");
  const version = requiredString(packageConfig.version, "package.version");
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error("package.version must be a semantic version");
  }
  return {
    productName: requiredString(packageConfig.productName, "package.productName"),
    version,
  };
}

function asRecord(value: unknown, field: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${field} must be an object`);
  }
  return value as Record<string, unknown>;
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}

function relativePath(value: unknown, field: string): string {
  const path = requiredString(value, field);
  if (path.startsWith("/") || path.split(/[\\/]/).includes("..")) {
    throw new Error(`${field} must be a relative path inside the project`);
  }
  return path;
}

function port(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < 1 || value > 65535) {
    throw new Error(`${field} must be an integer from 1 to 65535`);
  }
  return value;
}
