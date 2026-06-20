import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { CliPackageCommand, CliTopLevelCommand } from "./cli.js";
import { isCefariBuildTarget, type CefariBuildTarget } from "./platform.js";

export type ConfigMode = "development" | "production";

export interface CefariConfigContext {
  command: CliTopLevelCommand;
  packageCommand: CliPackageCommand | null;
  mode: ConfigMode;
  root: string;
}

export interface CefariConfigInput {
  app: AppConfigInput;
  browser?: BrowserConfigInput;
  capabilities?: CefariCapability[];
  workers?: Record<string, WorkerConfigInput>;
  vite?: ViteConfigInput;
  daemon?: DaemonConfigInput;
  targets?: CefariTargetsConfigInput;
  package: PackageConfigInput;
  frontend?: never;
}

export type CefariConfigExport =
  | CefariConfigInput
  | ((
    context: CefariConfigContext,
  ) => CefariConfigInput | Promise<CefariConfigInput>);

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

export type WorkerPermissionValueInput = "none" | string[];

export interface WorkerPermissionsInput {
  read?: WorkerPermissionValueInput;
  write?: WorkerPermissionValueInput;
  net?: WorkerPermissionValueInput;
  env?: WorkerPermissionValueInput;
  run?: WorkerPermissionValueInput;
  ffi?: WorkerPermissionValueInput;
}

export interface WorkerConfigInput {
  entry: string;
  permissions: WorkerPermissionsInput;
  native?: WorkerNativePayloadInput[];
}

export interface WorkerNativePayloadInput {
  src: string;
  target: string;
  platforms?: CefariBuildTarget[];
  executable?: boolean;
}

export interface PackageConfigInput {
  productName: string;
  version: string;
}

export type CefariAppTarget = "desktop" | "ios" | "android";

export interface CefariTargetsConfigInput {
  desktop?: DesktopTargetConfigInput;
  ios?: IosTargetConfigInput;
  android?: AndroidTargetConfigInput;
}

export interface DesktopTargetConfigInput {
  capabilities?: CefariCapability[];
  daemon?: DaemonConfigInput;
}

export interface IosTargetConfigInput {
  bundleId?: string;
  permissions?: string[];
}

export interface AndroidTargetConfigInput {
  applicationId?: string;
  permissions?: string[];
}

export interface BrowserConfigInput {
  webgpu?: boolean;
}

export interface TrayCapabilityOptions {
  icon: string;
}

export interface TrayCapabilityInput extends TrayCapabilityOptions {
  type: "tray";
}

export interface DeepLinksCapabilityOptions {
  schemes: string[];
}

export interface DeepLinksCapabilityInput extends DeepLinksCapabilityOptions {
  type: "deepLinks";
}

export type CefariCapability = TrayCapabilityInput | DeepLinksCapabilityInput;

export interface AppConfig extends AppConfigInput {}

export interface ViteConfig {
  root: string;
  configFile?: string | false;
  devPort: number;
}

export interface DaemonConfig extends DaemonConfigInput {}

export type WorkerPermissionValue = "none" | string[];

export interface WorkerPermissions {
  read: WorkerPermissionValue;
  write: WorkerPermissionValue;
  net: WorkerPermissionValue;
  env: WorkerPermissionValue;
  run: WorkerPermissionValue;
  ffi: WorkerPermissionValue;
}

export interface WorkerConfig {
  entry: string;
  permissions: WorkerPermissions;
  native: WorkerNativePayload[];
}

export interface WorkerNativePayload {
  src: string;
  target: string;
  platforms: CefariBuildTarget[];
  executable: boolean;
}

export interface PackageConfig extends PackageConfigInput {}

export interface CefariTargetsConfig {
  desktop: DesktopTargetConfig;
  ios?: IosTargetConfig;
  android?: AndroidTargetConfig;
}

export interface DesktopTargetConfig {
  capabilities: CefariCapability[];
  daemon?: DaemonConfig;
}

export interface IosTargetConfig {
  bundleId: string;
  permissions: string[];
}

export interface AndroidTargetConfig {
  applicationId: string;
  permissions: string[];
}

export interface BrowserConfig {
  webgpu: boolean;
}

export interface ResolvedCefariConfig {
  root: string;
  configPath: string;
  app: AppConfig;
  browser: BrowserConfig;
  capabilities: CefariCapability[];
  workers: Record<string, WorkerConfig>;
  vite: ViteConfig;
  daemon?: DaemonConfig;
  targets: CefariTargetsConfig;
  package: PackageConfig;
}

export interface SerializableProjectConfig {
  app: AppConfig;
  browser: BrowserConfig;
  capabilities: CefariCapability[];
  workers: Record<string, WorkerConfig>;
  vite: ViteConfig;
  daemon?: DaemonConfig;
  targets: CefariTargetsConfig;
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

export function deepLinks(
  config: DeepLinksCapabilityOptions,
): DeepLinksCapabilityInput {
  return { type: "deepLinks", ...config };
}

export async function loadCefariConfig(
  options: LoadCefariConfigOptions = {},
): Promise<ResolvedCefariConfig> {
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

  const { runnerImport } = await import("vite");
  const { module } = await runnerImport<CefariConfigModule>(
    pathToFileURL(configPath).href,
    {
      root,
      logLevel: "silent",
      resolve: {
        alias: [{
          find: /^cefari$/,
          replacement: resolve(
            dirname(fileURLToPath(import.meta.url)),
            "index.js",
          ),
        }],
      },
    },
  );

  if (!Object.hasOwn(module, "default")) {
    throw new Error(
      `Cefari config at ${configPath} must have a default export`,
    );
  }

  const configExport = module.default;
  const config = typeof configExport === "function"
    ? await configExport(context)
    : configExport;
  return normalizeConfig(config, { root, configPath });
}

export function toSerializableProjectConfig(
  config: ResolvedCefariConfig,
): SerializableProjectConfig {
  return {
    app: { ...config.app },
    browser: { ...config.browser },
    capabilities: config.capabilities.map((capability) => ({ ...capability })),
    workers: cloneWorkers(config.workers),
    vite: { ...config.vite },
    ...(config.daemon === undefined ? {} : { daemon: { ...config.daemon } }),
    targets: cloneTargets(config.targets),
    package: { ...config.package },
  };
}

function normalizeConfig(
  config: unknown,
  paths: { root: string; configPath: string },
): ResolvedCefariConfig {
  const input = asRecord(config, "default export");
  if ("frontend" in input) {
    throw new Error(
      "frontend is no longer supported in cefari.config.ts; use vite instead",
    );
  }

  const app = normalizeApp(input.app);
  const browser = normalizeBrowser(input.browser);
  const topLevelCapabilities = normalizeCapabilities(
    input.capabilities,
    "capabilities",
  );
  const workers = normalizeWorkers(input.workers);
  const vite = normalizeVite(input.vite);
  const topLevelDaemon = normalizeDaemon(input.daemon, "daemon");
  const targets = normalizeTargets(input.targets, {
    app,
    topLevelCapabilities,
    topLevelDaemon,
  });
  const capabilities = targets.desktop.capabilities;
  const daemon = targets.desktop.daemon;
  const packageConfig = normalizePackage(input.package);

  return {
    root: paths.root,
    configPath: paths.configPath,
    app,
    browser,
    capabilities,
    workers,
    vite,
    daemon,
    targets,
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
    ...(app.icon === undefined
      ? {}
      : { icon: relativePath(app.icon, "app.icon") }),
  };
}

function normalizeBrowser(value: unknown): BrowserConfig {
  if (value === undefined) {
    return {
      webgpu: false,
    };
  }
  const browser = asRecord(value, "browser");
  return {
    webgpu: optionalBoolean(browser.webgpu, "browser.webgpu", false),
  };
}

function normalizeCapabilities(
  value: unknown,
  field: string,
): CefariCapability[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error(`${field} must be an array`);
  }

  let trayCount = 0;
  const deepLinkSchemes = new Set<string>();
  return value.map((entry, index) => {
    const capabilityField = `${field}[${index}]`;
    const capability = asRecord(entry, capabilityField);
    const type = requiredString(capability.type, `${capabilityField}.type`);
    if (type === "tray") {
      trayCount += 1;
      if (trayCount > 1) {
        throw new Error(
          `${field} must not include more than one tray capability`,
        );
      }
      return {
        type: "tray",
        icon: relativePath(capability.icon, `${capabilityField}.icon`),
      };
    }
    if (type === "deepLinks") {
      return {
        type: "deepLinks",
        schemes: normalizeDeepLinkSchemes(
          capability.schemes,
          `${capabilityField}.schemes`,
          deepLinkSchemes,
        ),
      };
    }
    throw new Error(`${capabilityField}.type must be "tray" or "deepLinks"`);
  });
}

function normalizeWorkers(value: unknown): Record<string, WorkerConfig> {
  if (value === undefined) {
    return {};
  }

  const workers = asRecord(value, "workers");
  const normalized: Record<string, WorkerConfig> = {};
  for (const [id, config] of Object.entries(workers)) {
    const field = `workers.${id}`;
    if (!/^[a-z][a-z0-9-]*$/.test(id)) {
      throw new Error(`${field} must use an id matching ^[a-z][a-z0-9-]*$`);
    }
    const worker = asRecord(config, field);
    assertOnlyFields(worker, field, ["entry", "permissions", "native"]);
    normalized[id] = {
      entry: relativePath(worker.entry, `${field}.entry`),
      permissions: normalizeWorkerPermissions(
        worker.permissions,
        `${field}.permissions`,
      ),
      native: normalizeWorkerNativePayloads(worker.native, `${field}.native`),
    };
  }
  return normalized;
}

function normalizeWorkerPermissions(
  value: unknown,
  field: string,
): WorkerPermissions {
  const permissions = asRecord(value, field);
  assertOnlyFields(permissions, field, ["read", "write", "net", "env", "run", "ffi"]);
  return {
    read: normalizeWorkerPermissionValue(
      permissions.read,
      `${field}.read`,
      "path",
    ),
    write: normalizeWorkerPermissionValue(
      permissions.write,
      `${field}.write`,
      "path",
    ),
    net: normalizeWorkerPermissionValue(
      permissions.net,
      `${field}.net`,
      "name",
    ),
    env: normalizeWorkerPermissionValue(
      permissions.env,
      `${field}.env`,
      "name",
    ),
    run: normalizeWorkerPermissionValue(
      permissions.run,
      `${field}.run`,
      "path",
    ),
    ffi: normalizeWorkerPermissionValue(permissions.ffi, `${field}.ffi`, "path"),
  };
}

function normalizeWorkerNativePayloads(value: unknown, field: string): WorkerNativePayload[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error(`${field} must be an array`);
  }

  const targets = new Set<string>();
  return value.map((entry, index) => {
    const payloadField = `${field}[${index}]`;
    const payload = asRecord(entry, payloadField);
    assertOnlyFields(payload, payloadField, ["src", "target", "platforms", "executable"]);
    const target = relativeResourcePath(payload.target, `${payloadField}.target`);
    if (targets.has(target)) {
      throw new Error(`${payloadField}.target duplicates worker native target "${target}"`);
    }
    targets.add(target);
    return {
      src: relativePath(payload.src, `${payloadField}.src`),
      target,
      platforms: normalizeWorkerNativePlatforms(payload.platforms, `${payloadField}.platforms`),
      executable: optionalBoolean(payload.executable, `${payloadField}.executable`, false),
    };
  });
}

function normalizeWorkerNativePlatforms(value: unknown, field: string): CefariBuildTarget[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${field} must be a non-empty Cefari build target array`);
  }
  return value.map((entry, index) => {
    const target = requiredString(entry, `${field}[${index}]`);
    if (!isCefariBuildTarget(target)) {
      throw new Error(`${field}[${index}] must be a supported Cefari build target`);
    }
    return target;
  });
}

function normalizeWorkerPermissionValue(
  value: unknown,
  field: string,
  itemKind: "path" | "name",
): WorkerPermissionValue {
  if (value === undefined || value === "none") {
    return "none";
  }
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${field} must be "none" or a non-empty string array`);
  }

  return value.map((entry, index) => {
    const item = requiredString(entry, `${field}[${index}]`);
    if (
      itemKind === "path" &&
      (item.startsWith("/") || item.split(/[\\/]/).includes(".."))
    ) {
      throw new Error(
        `${field}[${index}] must be a relative path or Cefari permission token`,
      );
    }
    return item;
  });
}

function normalizeDeepLinkSchemes(
  value: unknown,
  field: string,
  seen: Set<string>,
): string[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${field} must be a non-empty array`);
  }

  return value.map((scheme, index) => {
    const normalized = deepLinkScheme(scheme, `${field}[${index}]`);
    if (seen.has(normalized)) {
      throw new Error(
        `${field}[${index}] duplicates deep link scheme "${normalized}"`,
      );
    }
    seen.add(normalized);
    return normalized;
  });
}

function deepLinkScheme(value: unknown, field: string): string {
  const scheme = requiredString(value, field);
  if (scheme.includes("://")) {
    throw new Error(`${field} must be a URL scheme without ://`);
  }
  if (!/^[a-z][a-z0-9+.-]*$/.test(scheme)) {
    throw new Error(
      `${field} must be lowercase ASCII and match ^[a-z][a-z0-9+.-]*$`,
    );
  }
  if (["http", "https", "file", "mailto", "cefari"].includes(scheme)) {
    throw new Error(`${field} must not use reserved scheme "${scheme}"`);
  }
  return scheme;
}

function normalizeVite(value: unknown): ViteConfig {
  if (value === undefined) {
    return {
      root: "frontend",
      devPort: 5173,
    };
  }

  const vite = asRecord(value, "vite");
  const configFile = vite.configFile === undefined
    ? undefined
    : normalizeViteConfigFile(vite.configFile);
  return {
    root: vite.root === undefined
      ? "frontend"
      : relativePath(vite.root, "vite.root"),
    ...(configFile === undefined ? {} : { configFile }),
    devPort: vite.devPort === undefined
      ? 5173
      : port(vite.devPort, "vite.devPort"),
  };
}

function normalizeViteConfigFile(value: unknown): string | false {
  if (value === false) {
    return false;
  }
  return relativePath(value, "vite.configFile");
}

function normalizeDaemon(
  value: unknown,
  field: string,
): DaemonConfig | undefined {
  if (value === undefined) {
    return undefined;
  }
  const daemon = asRecord(value, field);
  return {
    entry: relativePath(daemon.entry, `${field}.entry`),
  };
}

function normalizeTargets(
  value: unknown,
  defaults: {
    app: AppConfig;
    topLevelCapabilities: CefariCapability[];
    topLevelDaemon?: DaemonConfig;
  },
): CefariTargetsConfig {
  if (value === undefined) {
    return {
      desktop: {
        capabilities: defaults.topLevelCapabilities,
        ...(defaults.topLevelDaemon === undefined
          ? {}
          : { daemon: defaults.topLevelDaemon }),
      },
    };
  }

  const targets = asRecord(value, "targets");
  assertOnlyFields(targets, "targets", ["desktop", "ios", "android"]);
  return {
    desktop: normalizeDesktopTarget(targets.desktop, defaults),
    ...(targets.ios === undefined
      ? {}
      : { ios: normalizeIosTarget(targets.ios, defaults.app) }),
    ...(targets.android === undefined
      ? {}
      : { android: normalizeAndroidTarget(targets.android, defaults.app) }),
  };
}

function normalizeDesktopTarget(
  value: unknown,
  defaults: {
    topLevelCapabilities: CefariCapability[];
    topLevelDaemon?: DaemonConfig;
  },
): DesktopTargetConfig {
  if (value === undefined) {
    return {
      capabilities: defaults.topLevelCapabilities,
      ...(defaults.topLevelDaemon === undefined
        ? {}
        : { daemon: defaults.topLevelDaemon }),
    };
  }

  const desktop = asRecord(value, "targets.desktop");
  assertOnlyFields(desktop, "targets.desktop", ["capabilities", "daemon"]);
  return {
    capabilities: desktop.capabilities === undefined
      ? defaults.topLevelCapabilities
      : normalizeCapabilities(
        desktop.capabilities,
        "targets.desktop.capabilities",
      ),
    daemon: desktop.daemon === undefined
      ? defaults.topLevelDaemon
      : normalizeDaemon(desktop.daemon, "targets.desktop.daemon"),
  };
}

function normalizeIosTarget(value: unknown, app: AppConfig): IosTargetConfig {
  const ios = asRecord(value, "targets.ios");
  assertOnlyFields(ios, "targets.ios", ["bundleId", "permissions"]);
  return {
    bundleId: optionalString(
      ios.bundleId,
      "targets.ios.bundleId",
      app.identifier,
    ),
    permissions: optionalStringArray(
      ios.permissions,
      "targets.ios.permissions",
    ),
  };
}

function normalizeAndroidTarget(
  value: unknown,
  app: AppConfig,
): AndroidTargetConfig {
  const android = asRecord(value, "targets.android");
  assertOnlyFields(android, "targets.android", [
    "applicationId",
    "permissions",
  ]);
  return {
    applicationId: optionalString(
      android.applicationId,
      "targets.android.applicationId",
      app.identifier,
    ),
    permissions: optionalStringArray(
      android.permissions,
      "targets.android.permissions",
    ),
  };
}

function normalizePackage(value: unknown): PackageConfig {
  const packageConfig = asRecord(value, "package");
  const version = requiredString(packageConfig.version, "package.version");
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error("package.version must be a semantic version");
  }
  return {
    productName: requiredString(
      packageConfig.productName,
      "package.productName",
    ),
    version,
  };
}

function asRecord(value: unknown, field: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${field} must be an object`);
  }
  return value as Record<string, unknown>;
}

function assertOnlyFields(
  value: Record<string, unknown>,
  field: string,
  allowedFields: string[],
): void {
  const allowed = new Set(allowedFields);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw new Error(`${field}.${key} is not supported`);
    }
  }
}

function cloneWorkers(
  workers: Record<string, WorkerConfig>,
): Record<string, WorkerConfig> {
  return Object.fromEntries(
    Object.entries(workers).map(([id, worker]) => [
      id,
      {
        entry: worker.entry,
        permissions: {
          read: clonePermissionValue(worker.permissions.read),
          write: clonePermissionValue(worker.permissions.write),
          net: clonePermissionValue(worker.permissions.net),
          env: clonePermissionValue(worker.permissions.env),
          run: clonePermissionValue(worker.permissions.run),
          ffi: clonePermissionValue(worker.permissions.ffi),
        },
        native: worker.native.map((payload) => ({
          src: payload.src,
          target: payload.target,
          platforms: [...payload.platforms],
          executable: payload.executable,
        })),
      },
    ]),
  );
}

function clonePermissionValue(
  value: WorkerPermissionValue,
): WorkerPermissionValue {
  return value === "none" ? "none" : [...value];
}

function cloneTargets(targets: CefariTargetsConfig): CefariTargetsConfig {
  return {
    desktop: {
      capabilities: targets.desktop.capabilities.map((capability) => ({
        ...capability,
      })),
      ...(targets.desktop.daemon === undefined
        ? {}
        : { daemon: { ...targets.desktop.daemon } }),
    },
    ...(targets.ios === undefined ? {} : {
      ios: {
        bundleId: targets.ios.bundleId,
        permissions: [...targets.ios.permissions],
      },
    }),
    ...(targets.android === undefined ? {} : {
      android: {
        applicationId: targets.android.applicationId,
        permissions: [...targets.android.permissions],
      },
    }),
  };
}

function requiredString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}

function optionalString(
  value: unknown,
  field: string,
  defaultValue: string,
): string {
  if (value === undefined) {
    return defaultValue;
  }
  return requiredString(value, field);
}

function optionalStringArray(value: unknown, field: string): string[] {
  if (value === undefined) {
    return [];
  }
  if (!Array.isArray(value)) {
    throw new Error(`${field} must be a string array`);
  }
  return value.map((entry, index) =>
    requiredString(entry, `${field}[${index}]`)
  );
}

function optionalBoolean(
  value: unknown,
  field: string,
  defaultValue: boolean,
): boolean {
  if (value === undefined) {
    return defaultValue;
  }
  if (typeof value !== "boolean") {
    throw new Error(`${field} must be a boolean`);
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

function relativeResourcePath(value: unknown, field: string): string {
  const path = requiredString(value, field);
  if (path.startsWith("/") || path.split(/[\\/]/).includes("..")) {
    throw new Error(`${field} must be a relative resource path`);
  }
  return path.replaceAll("\\", "/");
}

function port(value: unknown, field: string): number {
  if (
    typeof value !== "number" || !Number.isInteger(value) || value < 1 ||
    value > 65535
  ) {
    throw new Error(`${field} must be an integer from 1 to 65535`);
  }
  return value;
}
