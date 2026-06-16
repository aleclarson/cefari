import { join, resolve } from "node:path";
import type { CommandRunner } from "./command.ts";
import type { ReleaseConfig } from "./inputs.ts";

export function inferUpdateTarget(config: ReleaseConfig): string {
  return config.updateTarget;
}

export async function createUpdateInputArchive(
  config: ReleaseConfig,
  target: string,
  runCommand: CommandRunner,
): Promise<string> {
  const archive = join(config.updateInputDir, `${target}.zip`);
  const archivePath = resolve(archive);
  await Deno.remove(config.updateInputDir, { recursive: true }).catch(ignoreNotFound);
  await Deno.mkdir(config.updateInputDir, { recursive: true });
  await Deno.remove(archivePath).catch(ignoreNotFound);
  await runCommand(["zip", "-qr", archivePath, "."], { cwd: join(config.packageDir, "output") });
  return archivePath;
}

function ignoreNotFound(error: unknown): void {
  if (!(error instanceof Deno.errors.NotFound)) {
    throw error;
  }
}
