import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import type { SpawnOptions } from "node:child_process";
import { loadCefariConfig } from "./config.js";
import type { ResolvedCefariConfig } from "./config.js";
import { runCefariBuild } from "./build.js";

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

export interface PackageDependencies {
  spawnSync(command: string, args: string[], options: SpawnOptions): { status: number | null; error?: Error };
  env: NodeJS.ProcessEnv;
  stdout: Pick<NodeJS.WriteStream, "write">;
}

export async function runCefariPackage(options: PackageOptions = {}, deps = defaultPackageDependencies()): Promise<void> {
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

  ensureBuildArtifacts(config, buildDir, cefResources);
  await mkdir(packageDir, { recursive: true });
  await writePackageMetadata(config, packageDir, buildDir, cefResources, options.releaseVersion);
  await writePackageManifest(config, packageDir, buildDir, cefResources);
  deps.stdout.write(`prepared package assembly at ${packageDir}\n`);
  runCargoPackager(packageDir, deps);
}

export function runPackageSign(options: SignOptions, deps = defaultPackageDependencies()): void {
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
  runCommand("cargo-codesign", args, deps, "cargo-codesign codesign");
  deps.stdout.write(`signed artifact at ${options.artifact}\n`);
}

export function runPackageNotarize(options: NotarizeOptions, deps = defaultPackageDependencies()): void {
  ensureArtifact(options.artifact);
  const args = ["codesign"];
  pushConfig(args, options.config);
  args.push("macos");
  pushMacosArtifact(args, options.artifact);
  runCommand("cargo-codesign", args, deps, "cargo-codesign notarize");
  deps.stdout.write(`notarized artifact at ${options.artifact}\n`);
}

export async function runPackageUpdate(options: UpdateOptions, deps = defaultPackageDependencies()): Promise<void> {
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
    deps,
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
  deps.stdout.write(`generated update artifacts at ${outputDir}\n`);
}

export async function runPackageRelease(options: ReleaseOptions = {}, deps = defaultPackageDependencies()): Promise<void> {
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
  deps.stdout.write(`Cefari release plan\n  mode: ${mode}\n  version: ${version}\n`);
  if (options.dryRun) {
    deps.stdout.write("dry-run: build, package, signing, update, and GitHub release steps skipped\n");
    return;
  }
  await runCefariBuild({ root, release: true });
  await runCefariPackage({ root, release: true, releaseVersion: version }, deps);
}

function ensureBuildArtifacts(config: ResolvedCefariConfig, buildDir: string, cefResources: string): void {
  for (const path of [
    join(buildDir, "frontend", "index.html"),
    join(buildDir, "config", "cefari.json"),
    join(buildDir, "daemon", daemonExecutableName(config)),
    join(buildDir, "desktop", desktopExecutableName(config)),
    join(cefResources, "archive.json"),
  ]) {
    ensureArtifact(path);
  }
}

async function writePackageMetadata(
  config: ResolvedCefariConfig,
  packageDir: string,
  buildDir: string,
  cefResources: string,
  releaseVersion: string | undefined,
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
  const metadata = [
    `name = ${tomlString(config.app.identifier)}`,
    `product_name = ${tomlString(config.package.productName)}`,
    `version = ${tomlString(releaseVersion ?? config.package.version)}`,
    `identifier = ${tomlString(config.app.identifier)}`,
    `formats = [${tomlString(defaultPackageFormat())}]`,
    `binaries_dir = ${tomlString(join(buildDir, "desktop"))}`,
    "",
    "[[binaries]]",
    `path = ${tomlString(desktopExecutableName(config))}`,
    "main = true",
    "",
    ...resourceToml(join(buildDir, "frontend"), "frontend"),
    ...resourceToml(join(buildDir, "config"), "config"),
    ...resourceToml(join(buildDir, "daemon"), "daemon"),
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
): Promise<void> {
  const manifest = {
    product_name: config.package.productName,
    identifier: config.app.identifier,
    tray_icon: config.capabilities.some((capability) => capability.type === "tray") ? "tray-icon.png" : null,
    desktop_binary: desktopExecutableName(config),
    frontend_dir: normalizePath(join(buildDir, "frontend")),
    config_file: normalizePath(join(buildDir, "config", "cefari.json")),
    daemon_dir: normalizePath(join(buildDir, "daemon")),
    daemon_executable: normalizePath(join(buildDir, "daemon", daemonExecutableName(config))),
    cef_resources: normalizePath(cefResources),
    cef_archive_json: normalizePath(join(cefResources, "archive.json")),
  };
  await writeFile(join(packageDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

function runCargoPackager(packageDir: string, deps: PackageDependencies): void {
  const config = join(packageDir, "cargo-packager.toml");
  const output = join(packageDir, "output");
  const command = deps.spawnSync("cargo-packager", ["--config", config, "--out-dir", output], {
    cwd: packageDir,
    env: deps.env,
    stdio: "inherit",
  });
  if (command.error !== undefined && command.error.message.includes("ENOENT")) {
    deps.stdout.write("cargo-packager not found; skipped native package invocation\n");
    return;
  }
  if (command.error !== undefined) {
    throw command.error;
  }
  if (command.status !== 0) {
    throw new Error(`cargo-packager failed with status ${command.status}`);
  }
}

function runCommand(command: string, args: string[], deps: PackageDependencies, description: string): void {
  const result = deps.spawnSync(command, args, { env: deps.env, stdio: "inherit" });
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
  return `${process.platform}-${process.arch}`;
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

function defaultPackageFormat(): string {
  if (process.platform === "darwin") {
    return "dmg";
  }
  if (process.platform === "win32") {
    return "nsis";
  }
  return "deb";
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

function defaultPackageDependencies(): PackageDependencies {
  return {
    spawnSync: (command, args, options) => spawnSync(command, args, options),
    env: process.env,
    stdout: process.stdout,
  };
}
