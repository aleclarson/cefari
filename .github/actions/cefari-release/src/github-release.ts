import { prepareGitHubReleaseAssets } from "./artifacts.ts";
import { type CommandRunner, commandSucceeds, validateCommandAvailable } from "./command.ts";
import type { ReleaseConfig } from "./inputs.ts";

export async function publishGitHubRelease(
  config: ReleaseConfig,
  releaseAssets: string[],
  runCommand: CommandRunner,
): Promise<void> {
  const releaseTag = config.releaseTag || Deno.env.get("GITHUB_REF_NAME") || "";
  if (config.dryRun) {
    console.log(`+ gh release upload/create for ${releaseTag}`);
    return;
  }

  if (!Deno.env.get("GH_TOKEN") && !Deno.env.get("GITHUB_TOKEN")) {
    throw new Error("GH_TOKEN or GITHUB_TOKEN is required when create-github-release is true");
  }

  await validateCommandAvailable("gh", config.dryRun);
  const githubReleaseAssets = await prepareGitHubReleaseAssets(config, releaseAssets, runCommand);
  const releaseExists = await commandSucceeds(["gh", "release", "view", releaseTag]);
  if (releaseExists) {
    console.log(`GitHub release already exists: ${releaseTag}`);
  } else {
    const createReleaseArgs = [
      "gh",
      "release",
      "create",
      releaseTag,
      "--title",
      config.releaseName || releaseTag,
    ];
    if (config.mode === "prerelease") {
      createReleaseArgs.push("--prerelease");
    }
    await runCommand(createReleaseArgs);
  }

  await runCommand(["gh", "release", "upload", releaseTag, ...githubReleaseAssets, "--clobber"]);
}
