import { basename, dirname, join } from "node:path";
import type { CommandRunner } from "./command.ts";
import type { ReleaseConfig } from "./inputs.ts";

export function isReleaseArtifact(path: string): boolean {
  return [
    ".app",
    ".dmg",
    ".app.tar.gz",
    ".AppImage",
    ".deb",
    ".rpm",
    ".exe",
    ".msi",
    ".zip",
    ".tar.gz",
  ].some((suffix) => path.endsWith(suffix));
}

export function isSignableArtifact(path: string, platform: string): boolean {
  if (platform === "macos") {
    return path.endsWith(".app") || path.endsWith(".dmg");
  }
  if (platform === "linux") {
    return [".AppImage", ".deb", ".rpm", ".tar.gz", ".zip"].some((suffix) => path.endsWith(suffix));
  }
  if (platform === "windows") {
    return path.endsWith(".exe") || path.endsWith(".msi") || path.endsWith(".zip");
  }
  return false;
}

export function isNotarizableArtifact(path: string): boolean {
  return path.endsWith(".app") || path.endsWith(".dmg");
}

export async function collectReleaseAssets(config: ReleaseConfig): Promise<string[]> {
  const outputDir = join(config.packageDir, "output");
  await assertDirectory(outputDir, `package output directory not found at ${outputDir}`);

  const artifacts: string[] = [];
  for await (const entry of Deno.readDir(outputDir)) {
    if (!entry.isFile && !entry.isDirectory) continue;
    const artifact = join(outputDir, entry.name);
    if (isReleaseArtifact(artifact)) {
      artifacts.push(artifact);
    }
  }
  artifacts.sort();

  if (artifacts.length === 0) {
    throw new Error(`no release artifacts found under ${outputDir}`);
  }

  await Deno.mkdir(dirname(config.releaseArtifactsFile), { recursive: true });
  await Deno.writeTextFile(config.releaseArtifactsFile, `${artifacts.join("\n")}\n`);
  for (const artifact of artifacts) {
    console.log(`collected release artifact: ${artifact}`);
  }

  return artifacts;
}

export async function prepareGitHubReleaseAssets(
  config: ReleaseConfig,
  releaseAssets: string[],
  runCommand: CommandRunner,
): Promise<string[]> {
  await Deno.remove(config.githubReleaseAssetsDir, { recursive: true }).catch(ignoreNotFound);
  await Deno.mkdir(config.githubReleaseAssetsDir, { recursive: true });

  const githubReleaseAssets: string[] = [];
  for (const artifact of releaseAssets) {
    const info = await Deno.stat(artifact);
    if (info.isFile) {
      githubReleaseAssets.push(artifact);
    } else if (info.isDirectory) {
      githubReleaseAssets.push(await archiveDirectoryArtifact(config, artifact, runCommand));
    }
  }

  const updateDirInfo = await Deno.stat(config.updateDir).catch((error) => {
    if (error instanceof Deno.errors.NotFound) return undefined;
    throw error;
  });
  if (updateDirInfo?.isDirectory) {
    const updateAssets = await listFiles(config.updateDir);
    githubReleaseAssets.push(...updateAssets.sort());
  }

  if (githubReleaseAssets.length === 0) {
    throw new Error("no uploadable GitHub release assets were prepared");
  }

  return githubReleaseAssets;
}

async function archiveDirectoryArtifact(
  config: ReleaseConfig,
  artifact: string,
  runCommand: CommandRunner,
): Promise<string> {
  const archiveName = `${basename(artifact)}.tar.gz`;
  const archivePath = join(config.githubReleaseAssetsDir, archiveName);
  await runCommand(["tar", "-czf", archivePath, "-C", dirname(artifact), basename(artifact)]);
  return archivePath;
}

async function listFiles(root: string): Promise<string[]> {
  const files: string[] = [];
  for await (const entry of Deno.readDir(root)) {
    const path = join(root, entry.name);
    if (entry.isFile) {
      files.push(path);
    } else if (entry.isDirectory) {
      files.push(...await listFiles(path));
    }
  }
  return files;
}

async function assertDirectory(path: string, message: string): Promise<void> {
  const info = await Deno.stat(path).catch((error) => {
    if (error instanceof Deno.errors.NotFound) return undefined;
    throw error;
  });
  if (!info?.isDirectory) {
    throw new Error(message);
  }
}

function ignoreNotFound(error: unknown): void {
  if (!(error instanceof Deno.errors.NotFound)) {
    throw error;
  }
}
