import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { loadCefariConfig } from "./config.js";
import type { ResolvedCefariConfig } from "./config.js";
import { runCefariBuild } from "./build.js";
import { selectedWorkerNativePayloads, workerNativePayloadBuildPath } from "./native-payloads.js";
import {
  currentPlatform,
  executableNameForTarget,
  hostCefariBuildTarget,
  parseCefariBuildTarget,
  type CefariBuildTarget,
} from "./platform.js";

export type SignPlatform = "macos" | "windows" | "linux";
export type UpdatePackageFormat = "app" | "appimage" | "nsis" | "wix";
export type ReleaseMode = "release" | "prerelease";

export interface PackageOptions {
  root?: string;
  release?: boolean;
  releaseVersion?: string;
}

export interface SignOptions {
  artifact: string;
  platform?: SignPlatform;
  config?: string;
}

export interface NotarizeOptions {
  artifact: string;
  config?: string;
}

export interface UpdateOptions {
  archive: string;
  url: string;
  version: string;
  target?: string;
  format?: UpdatePackageFormat;
  keyEnv?: string;
  outputDir?: string;
}

export interface ReleaseOptions extends PackageOptions {
  version?: string;
  mode?: ReleaseMode;
  signingConfig?: string;
  signingPlatform?: SignPlatform;
  notarize?: boolean;
  updateUrlBase?: string;
  updateTarget?: string;
  updateFormat?: UpdatePackageFormat;
  updateKeyEnv?: string;
  githubRelease?: boolean;
  releaseTag?: string;
  releaseName?: string;
  dryRun?: boolean;
}

export async function runCefariPackage(options: PackageOptions = {}): Promise<void> {
  const { stdout } = currentPlatform();
  const root = resolve(options.root ?? process.cwd());
  const config = await loadCefariConfig({
    root,
    command: "package",
    packageCommand: "package",
    mode: "production",
  });
  const buildDir = join(root, "build");
  const packageDir = join(root, "dist", "package");
  const cefResources = join(buildDir, "cef", "resources");
  const target = await readBuildTarget(buildDir);

  ensureBuildArtifacts(config, buildDir, cefResources, target);
  await mkdir(packageDir, { recursive: true });
  await writePackageMetadata(config, packageDir, buildDir, cefResources, options.releaseVersion, target);
  await writePackageManifest(config, packageDir, buildDir, cefResources, target);
  stdout.write(`prepared package assembly at ${packageDir}\n`);
  runCargoPackager(packageDir);
}

export function runPackageSign(options: SignOptions): void {
  const { stdout } = currentPlatform();
  const platform = options.platform ?? currentSignPlatform();
  validateSignPlatform(platform);
  ensureArtifact(options.artifact);
  const args = ["codesign"];
  pushConfig(args, options.config);
  if (platform === "macos") {
    args.push("macos");
    pushMacosArtifact(args, options.artifact);
    args.push("--skip-notarize");
  } else if (platform === "windows") {
    args.push("windows");
  } else {
    args.push("linux", "--archive", options.artifact);
  }
  runCommand("cargo-codesign", args, "cargo-codesign codesign");
  stdout.write(`signed artifact at ${options.artifact}\n`);
}

export function runPackageNotarize(options: NotarizeOptions): void {
  const { stdout } = currentPlatform();
  ensureArtifact(options.artifact);
  const args = ["codesign"];
  pushConfig(args, options.config);
  args.push("macos");
  pushMacosArtifact(args, options.artifact);
  runCommand("cargo-codesign", args, "cargo-codesign notarize");
  stdout.write(`notarized artifact at ${options.artifact}\n`);
}

export async function runPackageUpdate(options: UpdateOptions): Promise<void> {
  const { stdout } = currentPlatform();
  ensureArtifact(options.archive);
  const target = options.target ?? defaultUpdateTarget();
  const format = options.format ?? defaultUpdateFormat(target);
  validateUpdateFormat(format);
  const keyEnv = options.keyEnv ?? "UPDATE_SIGNING_KEY";
  const outputDir = resolve(options.outputDir ?? "dist/update");
  await mkdir(outputDir, { recursive: true });
  const signaturePath = join(outputDir, `${basename(options.archive)}.sig`);
  runCommand(
    "cargo-codesign",
    ["codesign", "update", "--archive", options.archive, "--output", signaturePath, "--key-env", keyEnv],
    "cargo-codesign update",
  );
  const signature = (await readFile(signaturePath, "utf8")).trim();
  await writeFile(
    join(outputDir, "update.json"),
    `${JSON.stringify(
      {
        version: options.version,
        platforms: {
          [target]: {
            format,
            signature,
            url: options.url,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
  stdout.write(`generated update artifacts at ${outputDir}\n`);
}

export async function runPackageRelease(options: ReleaseOptions = {}): Promise<void> {
  const { stdout } = currentPlatform();
  const root = resolve(options.root ?? process.cwd());
  const mode = options.mode ?? "release";
  if (mode !== "release" && mode !== "prerelease") {
    throw new Error("release mode must be release or prerelease");
  }
  const config = await loadCefariConfig({
    root,
    command: "package",
    packageCommand: "release",
    mode: "production",
  });
  const version = options.version ?? config.package.version;
  stdout.write(`Cefari release plan\n  mode: ${mode}\n  version: ${version}\n`);
  if (options.dryRun) {
    stdout.write("dry-run: build, package, signing, update, and GitHub release steps skipped\n");
    return;
  }
  await runCefariBuild({ root, release: true });
  await runCefariPackage({ root, release: true, releaseVersion: version });
}

function ensureBuildArtifacts(
  config: ResolvedCefariConfig,
  buildDir: string,
  cefResources: string,
  target: CefariBuildTarget,
): void {
  for (const path of [
    join(buildDir, "frontend", "index.html"),
    join(buildDir, "config", "cefari.json"),
    join(buildDir, "desktop", desktopExecutableName(config, target)),
    join(buildDir, "workers"),
    join(cefResources, "archive.json"),
    ...(config.daemon === undefined ? [] : [join(buildDir, "daemon", daemonExecutableName(config, target))]),
  ]) {
    ensureArtifact(path);
  }
  for (const worker of Object.keys(config.workers)) {
    ensureArtifact(join(buildDir, "workers", worker, executableNameForTarget(worker, target)));
  }
  for (const selected of selectedWorkerNativePayloads(config, target)) {
    ensureArtifact(workerNativePayloadBuildPath(buildDir, selected));
  }
}

async function writePackageMetadata(
  config: ResolvedCefariConfig,
  packageDir: string,
  buildDir: string,
  cefResources: string,
  releaseVersion: string | undefined,
  target: CefariBuildTarget,
): Promise<void> {
  const icon = config.app.icon === undefined ? undefined : resolve(config.root, config.app.icon);
  if (icon !== undefined) {
    ensureArtifact(icon);
  }
  const tray = config.capabilities.find((capability) => capability.type === "tray");
  if (tray !== undefined) {
    ensureArtifact(resolve(config.root, tray.icon));
  }
  const deepLinks = config.capabilities.filter((capability) => capability.type === "deepLinks");
  const notificationScheme = notificationProtocol(config.app.identifier);
  const metadata = [
    `name = ${tomlString(config.app.identifier)}`,
    `product_name = ${tomlString(config.package.productName)}`,
    `version = ${tomlString(releaseVersion ?? config.package.version)}`,
    `identifier = ${tomlString(config.app.identifier)}`,
    `formats = [${tomlString(defaultPackageFormat(target))}]`,
    `binaries_dir = ${tomlString(join(buildDir, "desktop"))}`,
    "",
    "[[deep_link_protocols]]",
    `schemes = [${tomlString(notificationScheme)}]`,
    `name = ${tomlString(`${config.app.identifier}.notification`)}`,
    "",
    "[[binaries]]",
    `path = ${tomlString(desktopExecutableName(config, target))}`,
    "main = true",
    "",
    ...resourceToml(join(buildDir, "frontend"), "frontend"),
    ...resourceToml(join(buildDir, "config"), "config"),
    ...(config.daemon === undefined ? [] : resourceToml(join(buildDir, "daemon"), "daemon")),
    ...resourceToml(join(buildDir, "workers"), "workers"),
    ...resourceToml(cefResources, "cef"),
    ...(tray === undefined ? [] : resourceToml(resolve(config.root, tray.icon), "tray-icon.png")),
    ...deepLinks.flatMap((capability) => deepLinkProtocolToml(capability.schemes)),
    ...(icon === undefined ? [] : ["", `icons = [${tomlString(icon)}]`]),
    "",
  ].join("\n");
  await writeFile(join(packageDir, "cargo-packager.toml"), metadata);
}

async function writePackageManifest(
  config: ResolvedCefariConfig,
  packageDir: string,
  buildDir: string,
  cefResources: string,
  target: CefariBuildTarget,
): Promise<void> {
  const manifest = {
    product_name: config.package.productName,
    identifier: config.app.identifier,
    notification_protocol: notificationProtocol(config.app.identifier),
    tray_icon: config.capabilities.some((capability) => capability.type === "tray") ? "tray-icon.png" : null,
    desktop_binary: desktopExecutableName(config, target),
    frontend_dir: normalizePath(join(buildDir, "frontend")),
    config_file: normalizePath(join(buildDir, "config", "cefari.json")),
    ...(config.daemon === undefined
      ? {}
      : {
          daemon_dir: normalizePath(join(buildDir, "daemon")),
          daemon_executable: normalizePath(join(buildDir, "daemon", daemonExecutableName(config, target))),
        }),
    workers_dir: normalizePath(join(buildDir, "workers")),
    worker_executables: Object.fromEntries(
      Object.keys(config.workers).map((worker) => [
        worker,
        normalizePath(join(buildDir, "workers", worker, executableNameForTarget(worker, target))),
      ]),
    ),
    worker_native_payloads: Object.fromEntries(
      Object.keys(config.workers).map((worker) => [
        worker,
        selectedWorkerNativePayloads(config, target)
          .filter((selected) => selected.worker === worker)
          .map((selected) => ({
            target: selected.payload.target,
            resource_path: selected.resourcePath,
            path: normalizePath(workerNativePayloadBuildPath(buildDir, selected)),
            executable: selected.payload.executable,
          })),
      ]),
    ),
    cef_resources: normalizePath(cefResources),
    cef_archive_json: normalizePath(join(cefResources, "archive.json")),
  };
  await writeFile(join(packageDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

function runCargoPackager(packageDir: string): void {
  const { env, spawnSync, stdout } = currentPlatform();
  const config = join(packageDir, "cargo-packager.toml");
  const output = join(packageDir, "output");
  const command = spawnSync("cargo-packager", ["--config", config, "--out-dir", output], {
    cwd: packageDir,
    env,
    stdio: "inherit",
  });
  if (command.error !== undefined && command.error.message.includes("ENOENT")) {
    stdout.write("cargo-packager not found; skipped native package invocation\n");
    return;
  }
  if (command.error !== undefined) {
    throw command.error;
  }
  if (command.status !== 0) {
    throw new Error(`cargo-packager failed with status ${command.status}`);
  }
}

function notificationProtocol(identifier: string): string {
  const slug = identifier.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return `cefari-notification-${slug || "app"}`;
}

function runCommand(command: string, args: string[], description: string): void {
  const { env, spawnSync } = currentPlatform();
  const result = spawnSync(command, args, { env, stdio: "inherit" });
  if (result.error !== undefined) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${description} failed with status ${result.status}`);
  }
}

function pushConfig(args: string[], config: string | undefined): void {
  if (config !== undefined) {
    args.push("--config", config);
  }
}

function pushMacosArtifact(args: string[], artifact: string): void {
  if (artifact.endsWith(".app")) {
    args.push("--app", artifact);
  } else if (artifact.endsWith(".dmg")) {
    args.push("--dmg", artifact);
  } else {
    throw new Error(`macOS signing requires a .app bundle or .dmg artifact: ${artifact}`);
  }
}

function ensureArtifact(path: string): void {
  if (!existsSync(path)) {
    throw new Error(`artifact does not exist: ${path}`);
  }
}

function currentSignPlatform(): SignPlatform {
  if (process.platform === "darwin") {
    return "macos";
  }
  if (process.platform === "win32") {
    return "windows";
  }
  return "linux";
}

function validateSignPlatform(platform: string): asserts platform is SignPlatform {
  if (platform !== "macos" && platform !== "windows" && platform !== "linux") {
    throw new Error("signing platform must be macos, windows, or linux");
  }
}

function validateUpdateFormat(format: string): asserts format is UpdatePackageFormat {
  if (format !== "app" && format !== "appimage" && format !== "nsis" && format !== "wix") {
    throw new Error("update format must be app, appimage, nsis, or wix");
  }
}

function defaultUpdateTarget(): string {
  return hostCefariBuildTarget();
}

function defaultUpdateFormat(target: string): UpdatePackageFormat {
  if (target.startsWith("macos-") || target.startsWith("darwin-")) {
    return "app";
  }
  if (target.startsWith("windows-")) {
    return "nsis";
  }
  return "appimage";
}

async function readBuildTarget(buildDir: string): Promise<CefariBuildTarget> {
  const manifestPath = join(buildDir, "cef", "manifest.json");
  if (!existsSync(manifestPath)) {
    return hostCefariBuildTarget();
  }

  const manifest = JSON.parse(await readFile(manifestPath, "utf8")) as {
    target?: string;
    target_os?: string;
    target_arch?: string;
  };
  if (manifest.target !== undefined) {
    return parseCefariBuildTarget(manifest.target);
  }
  if (manifest.target_os !== undefined && manifest.target_arch !== undefined) {
    const os = manifest.target_os === "win32" ? "windows" : manifest.target_os;
    return parseCefariBuildTarget(`${os}-${manifest.target_arch}`);
  }
  return hostCefariBuildTarget();
}

function defaultPackageFormat(target: CefariBuildTarget): string {
  if (target.startsWith("darwin-")) {
    return "dmg";
  }
  if (target.startsWith("windows-")) {
    return "nsis";
  }
  return "deb";
}

function daemonExecutableName(config: ResolvedCefariConfig, target: CefariBuildTarget): string {
  return executableNameForTarget(`${config.app.projectName}-daemon`, target);
}

function desktopExecutableName(config: ResolvedCefariConfig, target: CefariBuildTarget): string {
  return executableNameForTarget(config.app.projectName, target);
}

function resourceToml(src: string, target: string): string[] {
  return ["", "[[resources]]", `src = ${tomlString(src)}`, `target = ${tomlString(target)}`];
}

function deepLinkProtocolToml(schemes: string[]): string[] {
  return ["", "[[deep_link_protocols]]", `schemes = [${schemes.map(tomlString).join(", ")}]`];
}

function tomlString(value: string): string {
  return JSON.stringify(normalizePath(value));
}

function normalizePath(path: string): string {
  return path.replaceAll("\\", "/");
}
