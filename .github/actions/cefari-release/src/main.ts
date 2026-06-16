import { collectReleaseAssets, isNotarizableArtifact, isSignableArtifact } from "./artifacts.ts";
import { runCommand as runExternalCommand, validateCommandAvailable } from "./command.ts";
import { readPackageMetadataVersion, readProjectPackageVersion } from "./config-version.ts";
import { publishGitHubRelease } from "./github-release.ts";
import { readConfigFromEnv } from "./inputs.ts";
import { createUpdateInputArchive, inferUpdateTarget } from "./update.ts";

async function main(): Promise<void> {
  const config = await readConfigFromEnv();
  const runCommand = (args: string[], options = {}) =>
    runExternalCommand(args, config.dryRun, options);

  let effectiveVersion = config.releaseVersion;
  if (config.dryRun && !effectiveVersion) {
    effectiveVersion = await readProjectPackageVersion(config);
    if (!effectiveVersion) {
      throw new Error(
        "release-version was not provided and package.version could not be read from cefari.config.ts",
      );
    }
  }

  await writeOutputs(config);
  printPlan(config, effectiveVersion);

  if (config.installCli) {
    await validateCommandAvailable("pnpm", config.dryRun);
    await runCommand(["pnpm", "add", "-g", `cefari@${config.cefariVersion}`]);
  }

  await validateCommandAvailable(config.cefariCommand, config.dryRun);

  const releaseArgs = [
    config.cefariCommand,
    "package",
    "release",
    config.projectPath,
    "--mode",
    config.mode,
  ];
  if (config.releaseVersion) releaseArgs.push("--version", config.releaseVersion);
  if (config.signingPlatform) releaseArgs.push("--signing-platform", config.signingPlatform);
  if (config.signingConfig) releaseArgs.push("--signing-config", config.signingConfig);
  if (config.notarize) releaseArgs.push("--notarize");
  if (config.updateUrlBase) releaseArgs.push("--update-url-base", config.updateUrlBase);
  if (config.updateTarget) releaseArgs.push("--update-target", config.updateTarget);
  if (config.updateFormat) releaseArgs.push("--update-format", config.updateFormat);
  if (config.updateKeyEnv) releaseArgs.push("--update-key-env", config.updateKeyEnv);
  if (config.createGitHubRelease) releaseArgs.push("--github-release");
  if (config.releaseTag) releaseArgs.push("--release-tag", config.releaseTag);
  if (config.releaseName) releaseArgs.push("--release-name", config.releaseName);
  if (config.dryRun) releaseArgs.push("--dry-run");

  await runCommand([config.cefariCommand, "build", config.projectPath, "--release"]);
  await runCommand(releaseArgs);

  let releaseAssets: string[] = [];
  if (config.dryRun) {
    console.log("artifact collection skipped in dry-run");
  } else {
    effectiveVersion = await readPackageMetadataVersion(config);
    if (!effectiveVersion) {
      throw new Error("package metadata did not contain a version");
    }
    releaseAssets = await collectReleaseAssets(config);
  }

  await runSigning(config, releaseAssets, runCommand);
  await runNotarization(config, releaseAssets, runCommand);
  await runUpdate(config, effectiveVersion, runCommand);

  if (config.createGitHubRelease) {
    await publishGitHubRelease(config, releaseAssets, runCommand);
  } else {
    console.log("GitHub release creation skipped");
  }
}

async function writeOutputs(config: Awaited<ReturnType<typeof readConfigFromEnv>>): Promise<void> {
  const outputFile = Deno.env.get("GITHUB_OUTPUT");
  if (!outputFile) return;
  await Deno.writeTextFile(
    outputFile,
    [
      `package-dir=${config.packageDir}`,
      `update-dir=${config.updateDir}`,
      `artifact-dir=${config.artifactDir}`,
      `release-artifacts=${config.releaseArtifactsFile}`,
      `release-mode=${config.mode}`,
      "",
    ].join("\n"),
    { append: true },
  );
}

function printPlan(
  config: Awaited<ReturnType<typeof readConfigFromEnv>>,
  effectiveVersion: string,
): void {
  console.log("Cefari release plan");
  console.log(`  project: ${config.projectPath}`);
  console.log(`  mode: ${config.mode}`);
  console.log(`  version: ${effectiveVersion || "from package metadata"}`);
  console.log(`  targets: ${config.targets || "current runner"}`);
  console.log(`  cefari command: ${config.cefariCommand}`);
  console.log(`  install cli: ${config.installCli}`);
  console.log(`  dry-run: ${config.dryRun}`);
}

async function runSigning(
  config: Awaited<ReturnType<typeof readConfigFromEnv>>,
  releaseAssets: string[],
  runCommand: (args: string[]) => Promise<void>,
): Promise<void> {
  if (!config.signingConfig && !config.signingPlatform) {
    console.log("signing skipped: no signing platform or signing config provided");
    return;
  }
  if (config.dryRun) {
    console.log("signing skipped in dry-run: release artifacts are not collected");
    return;
  }

  const effectiveSigningPlatform = normalizeSigningPlatform(
    config.signingPlatform || Deno.build.os,
  );
  for (const asset of releaseAssets) {
    if (isSignableArtifact(asset, effectiveSigningPlatform)) {
      const signArgs = [config.cefariCommand, "package", "sign", asset];
      if (config.signingPlatform) signArgs.push("--platform", config.signingPlatform);
      if (config.signingConfig) signArgs.push("--config", config.signingConfig);
      await runCommand(signArgs);
    } else {
      console.log(`signing skipped for unsupported artifact: ${asset}`);
    }
  }
}

async function runNotarization(
  config: Awaited<ReturnType<typeof readConfigFromEnv>>,
  releaseAssets: string[],
  runCommand: (args: string[]) => Promise<void>,
): Promise<void> {
  if (!config.notarize) {
    console.log("notarization skipped");
    return;
  }
  if (config.dryRun) {
    console.log("notarization skipped in dry-run: release artifacts are not collected");
    return;
  }

  for (const asset of releaseAssets) {
    if (isNotarizableArtifact(asset)) {
      const notarizeArgs = [config.cefariCommand, "package", "notarize", asset];
      if (config.signingConfig) notarizeArgs.push("--config", config.signingConfig);
      await runCommand(notarizeArgs);
    } else {
      console.log(`notarization skipped for unsupported artifact: ${asset}`);
    }
  }
}

async function runUpdate(
  config: Awaited<ReturnType<typeof readConfigFromEnv>>,
  effectiveVersion: string,
  runCommand: (args: string[]) => Promise<void>,
): Promise<void> {
  if (!config.updateUrlBase) {
    console.log("update metadata skipped: update-url-base not provided");
    return;
  }
  if (!Deno.env.get(config.updateKeyEnv) && !config.dryRun) {
    console.log(`update metadata skipped: ${config.updateKeyEnv} is not set`);
    return;
  }

  const effectiveUpdateTarget = inferUpdateTarget(config);
  if (!effectiveUpdateTarget) {
    throw new Error("update-target is required when update-url-base is set");
  }

  const archive = config.dryRun
    ? `${config.updateInputDir}/${effectiveUpdateTarget}.zip`
    : await createUpdateInputArchive(config, effectiveUpdateTarget, runCommand);
  const archiveName = archive.split("/").at(-1) ?? archive;
  const updateUrl = `${config.updateUrlBase.replace(/\/+$/, "")}/${archiveName}`;
  const updateArgs = [
    config.cefariCommand,
    "package",
    "update",
    archive,
    "--url",
    updateUrl,
    "--version",
    effectiveVersion,
    "--key-env",
    config.updateKeyEnv,
    "--output-dir",
    config.updateDir,
    "--target",
    effectiveUpdateTarget,
  ];
  if (config.updateFormat) updateArgs.push("--format", config.updateFormat);
  await runCommand(updateArgs);
}

function normalizeSigningPlatform(platform: string): string {
  if (platform === "darwin") return "macos";
  if (["windows", "win32"].includes(platform)) return "windows";
  return platform;
}

if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`cefari-release: ${message}`);
    Deno.exit(1);
  }
}
