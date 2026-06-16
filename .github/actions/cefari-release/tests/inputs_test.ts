import { strict as assert } from "node:assert";
import { readBool, type ReleaseConfig, validateConfig } from "../src/inputs.ts";

Deno.test("readBool accepts only action boolean strings", () => {
  assert.equal(readBool("dry-run", "true"), true);
  assert.equal(readBool("dry-run", "false"), false);
  assert.throws(() => readBool("dry-run", "yes"), /dry-run must be true or false/);
});

Deno.test("validateConfig rejects incompatible update input", async () => {
  const root = await Deno.makeTempDir();
  try {
    await Deno.writeTextFile(`${root}/cefari.config.ts`, "export default {};\n");
    const config = testConfig(root, { updateUrlBase: "https://example.test/releases" });
    await assert.rejects(
      () => validateConfig(config),
      /update-target is required when update-url-base is set/,
    );
  } finally {
    await Deno.remove(root, { recursive: true });
  }
});

Deno.test("validateConfig rejects notarization without macos signing config", async () => {
  const root = await Deno.makeTempDir();
  try {
    await Deno.writeTextFile(`${root}/cefari.config.ts`, "export default {};\n");
    const config = testConfig(root, { notarize: true, signingPlatform: "linux" });
    await assert.rejects(
      () => validateConfig(config),
      /signing-platform must be macos when notarize is true/,
    );
  } finally {
    await Deno.remove(root, { recursive: true });
  }
});

function testConfig(root: string, overrides: Partial<ReleaseConfig> = {}): ReleaseConfig {
  return {
    projectPath: root,
    mode: "release",
    targets: "",
    cefariCommand: "cefari",
    installCli: false,
    cefariVersion: "",
    releaseVersion: "",
    releaseTag: "",
    releaseName: "",
    createGitHubRelease: false,
    uploadArtifacts: true,
    artifactName: "cefari-release",
    signingPlatform: "",
    signingConfig: "",
    notarize: false,
    updateUrlBase: "",
    updateTarget: "",
    updateFormat: "",
    updateKeyEnv: "UPDATE_SIGNING_KEY",
    dryRun: false,
    packageDir: `${root}/dist/package`,
    updateDir: `${root}/dist/update`,
    artifactDir: `${root}/dist`,
    releaseArtifactsFile: `${root}/dist/release-artifacts.txt`,
    githubReleaseAssetsDir: `${root}/dist/github-release-assets`,
    updateInputDir: `${root}/dist/update-input`,
    ...overrides,
  };
}
