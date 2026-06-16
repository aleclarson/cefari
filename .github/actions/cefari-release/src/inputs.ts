import { join } from "node:path";

export type ReleaseMode = "release" | "prerelease";

export interface ReleaseConfig {
  projectPath: string;
  mode: ReleaseMode;
  targets: string;
  cefariCommand: string;
  installCli: boolean;
  cefariVersion: string;
  releaseVersion: string;
  releaseTag: string;
  releaseName: string;
  createGitHubRelease: boolean;
  uploadArtifacts: boolean;
  artifactName: string;
  signingPlatform: string;
  signingConfig: string;
  notarize: boolean;
  updateUrlBase: string;
  updateTarget: string;
  updateFormat: string;
  updateKeyEnv: string;
  dryRun: boolean;
  packageDir: string;
  updateDir: string;
  artifactDir: string;
  releaseArtifactsFile: string;
  githubReleaseAssetsDir: string;
  updateInputDir: string;
}

export async function readConfigFromEnv(): Promise<ReleaseConfig> {
  const projectPath = env("CEFARI_PROJECT_PATH", ".");
  const mode = readMode(env("CEFARI_RELEASE_MODE", "release"));
  const config: ReleaseConfig = {
    projectPath,
    mode,
    targets: env("CEFARI_TARGETS"),
    cefariCommand: env("CEFARI_COMMAND", "cefari"),
    installCli: readBool("install-cli", env("CEFARI_INSTALL_CLI", "false")),
    cefariVersion: env("CEFARI_CLI_VERSION"),
    releaseVersion: env("CEFARI_RELEASE_VERSION"),
    releaseTag: env("CEFARI_RELEASE_TAG"),
    releaseName: env("CEFARI_RELEASE_NAME"),
    createGitHubRelease: readBool(
      "create-github-release",
      env("CEFARI_CREATE_GITHUB_RELEASE", "false"),
    ),
    uploadArtifacts: readBool("upload-artifacts", env("CEFARI_UPLOAD_ARTIFACTS", "true")),
    artifactName: env("CEFARI_ARTIFACT_NAME", "cefari-release"),
    signingPlatform: env("CEFARI_SIGNING_PLATFORM"),
    signingConfig: env("CEFARI_SIGNING_CONFIG"),
    notarize: readBool("notarize", env("CEFARI_NOTARIZE", "false")),
    updateUrlBase: env("CEFARI_UPDATE_URL_BASE"),
    updateTarget: env("CEFARI_UPDATE_TARGET"),
    updateFormat: env("CEFARI_UPDATE_FORMAT"),
    updateKeyEnv: env("CEFARI_UPDATE_KEY_ENV", "UPDATE_SIGNING_KEY"),
    dryRun: readBool("dry-run", env("CEFARI_DRY_RUN", "false")),
    packageDir: join(projectPath, "dist", "package"),
    updateDir: join(projectPath, "dist", "update"),
    artifactDir: join(projectPath, "dist"),
    releaseArtifactsFile: join(projectPath, "dist", "release-artifacts.txt"),
    githubReleaseAssetsDir: join(projectPath, "dist", "github-release-assets"),
    updateInputDir: join(projectPath, "dist", "update-input"),
  };

  await validateConfig(config);
  return config;
}

export async function validateConfig(config: ReleaseConfig): Promise<void> {
  if (!config.cefariCommand) {
    throw new Error("cefari-command is required");
  }
  if (config.installCli && !config.cefariVersion) {
    throw new Error("cefari-version is required when install-cli is true");
  }
  if (!await exists(join(config.projectPath, "cefari.config.ts"))) {
    throw new Error(`cefari.config.ts not found at ${config.projectPath}`);
  }
  if (config.updateUrlBase && !config.updateTarget) {
    throw new Error("update-target is required when update-url-base is set");
  }
  if (config.signingConfig && !config.signingPlatform) {
    throw new Error("signing-platform is required when signing-config is set");
  }
  if (config.notarize && config.signingPlatform !== "macos") {
    throw new Error("signing-platform must be macos when notarize is true");
  }
  if (config.notarize && !config.signingConfig) {
    throw new Error("signing-config is required when notarize is true");
  }
  if (
    config.createGitHubRelease && !config.releaseTag && !Deno.env.get("GITHUB_REF_NAME")
  ) {
    throw new Error(
      "release-tag or GITHUB_REF_NAME is required when create-github-release is true",
    );
  }
}

export function readBool(name: string, value: string): boolean {
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error(`${name} must be true or false`);
}

function readMode(value: string): ReleaseMode {
  if (value === "release" || value === "prerelease") {
    return value;
  }
  throw new Error("mode must be release or prerelease");
}

function env(name: string, fallback = ""): string {
  return Deno.env.get(name) ?? fallback;
}

async function exists(path: string): Promise<boolean> {
  try {
    await Deno.stat(path);
    return true;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) return false;
    throw error;
  }
}
