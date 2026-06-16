import { binary, command, flag, number, option, optional, positional, run, string, subcommands } from "cmd-ts";
import { runCefariBuild } from "./build.js";
import { runCefariDev } from "./dev.js";
import {
  runCefariPackage,
  runPackageNotarize,
  runPackageRelease,
  runPackageSign,
  runPackageUpdate,
  type ReleaseMode,
  type SignPlatform,
  type UpdatePackageFormat,
} from "./package.js";

export const VERSION = "0.1.0";

export type CliTopLevelCommand = "dev" | "build" | "package" | "unknown";
export type CliPackageCommand = "package" | "sign" | "notarize" | "update" | "release";

type PlaceholderCommand =
  | "dev"
  | "build"
  | "package"
  | "package sign"
  | "package notarize"
  | "package update"
  | "package release";

function placeholder(commandName: PlaceholderCommand): void {
  console.log(`${commandName} is not implemented yet.`);
}

const projectPath = positional({
  type: optional(string),
  displayName: "project",
  description: "Project directory. Defaults to the current working directory.",
});

const release = flag({
  long: "release",
  description: "Use release-mode build or package behavior.",
  defaultValue: () => false,
});

const releaseVersion = option({
  type: optional(string),
  long: "release-version",
  description: "Version to write into release metadata.",
});

const version = option({
  type: optional(string),
  long: "version",
  description: "Version to write into release or update metadata.",
});

const artifact = positional({
  type: optional(string),
  displayName: "artifact",
  description: "Package artifact path.",
});

const url = option({
  type: optional(string),
  long: "url",
  description: "Download URL for update metadata.",
});

const configPath = option({
  type: optional(string),
  long: "config",
  description: "Path to signing configuration.",
});

const platform = option({
  type: optional(string),
  long: "platform",
  description: "Signing platform: macos, windows, or linux.",
});

const updateTarget = option({
  type: optional(string),
  long: "target",
  description: "Updater target key.",
});

const updateFormat = option({
  type: optional(string),
  long: "format",
  description: "Updater package format.",
});

const keyEnv = option({
  type: optional(string),
  long: "key-env",
  description: "Environment variable containing the update signing key.",
});

const releaseUpdateKeyEnv = option({
  type: optional(string),
  long: "update-key-env",
  description: "Environment variable containing the update signing key.",
});

const outputDir = option({
  type: optional(string),
  long: "output-dir",
  description: "Directory where update artifacts are written.",
});

const dev = command({
  name: "dev",
  description: "Run the Cefari app with Vite-powered development services.",
  args: {
    path: projectPath,
    vitePort: option({
      type: optional(number),
      long: "vite-port",
      description: "Override the fixed Vite dev server port.",
    }),
    devtoolsPort: option({
      type: optional(number),
      long: "devtools-port",
      description: "Chrome DevTools Protocol port for the embedded CEF browser.",
    }),
  },
  handler: async ({ path, vitePort, devtoolsPort }) => {
    await runCefariDev({ root: path, vitePort, devtoolsPort });
  },
});

const build = command({
  name: "build",
  description: "Build frontend, daemon, and desktop artifacts.",
  args: {
    path: projectPath,
    release,
  },
  handler: async ({ path, release }) => {
    await runCefariBuild({ root: path, release });
  },
});

const packageApp = command({
  name: "package",
  description: "Create a native package assembly.",
  args: {
    path: projectPath,
    release,
    releaseVersion,
  },
  handler: async ({ path, release, releaseVersion }) => {
    await runCefariPackage({ root: path, release, releaseVersion });
  },
});

const packageSign = command({
  name: "sign",
  description: "Sign packaged release artifacts.",
  args: {
    artifact,
    config: configPath,
    platform,
  },
  handler: ({ artifact, config, platform }) => {
    if (artifact === undefined) {
      throw new Error("package sign requires an artifact");
    }
    runPackageSign({ artifact, config, platform: platform as SignPlatform | undefined });
  },
});

const packageNotarize = command({
  name: "notarize",
  description: "Notarize packaged macOS artifacts.",
  args: {
    artifact,
    config: configPath,
  },
  handler: ({ artifact, config }) => {
    if (artifact === undefined) {
      throw new Error("package notarize requires an artifact");
    }
    runPackageNotarize({ artifact, config });
  },
});

const packageUpdate = command({
  name: "update",
  description: "Create updater metadata for a release artifact.",
  args: {
    archive: artifact,
    url,
    version,
    target: updateTarget,
    format: updateFormat,
    keyEnv,
    outputDir,
  },
  handler: async ({ archive, url, version, target, format, keyEnv, outputDir }) => {
    if (archive === undefined || url === undefined || version === undefined) {
      throw new Error("package update requires archive, --url, and --version");
    }
    await runPackageUpdate({
      archive,
      url,
      version,
      target,
      format: format as UpdatePackageFormat | undefined,
      keyEnv,
      outputDir,
    });
  },
});

const packageRelease = command({
  name: "release",
  description: "Run the release packaging pipeline.",
  args: {
    path: projectPath,
    version,
    mode: option({
      type: optional(string),
      long: "mode",
      description: "Release mode: release or prerelease.",
    }),
    signingConfig: option({
      type: optional(string),
      long: "signing-config",
      description: "Path to signing configuration.",
    }),
    signingPlatform: platform,
    notarize: flag({
      long: "notarize",
      description: "Notarize macOS artifacts.",
      defaultValue: () => false,
    }),
    updateUrlBase: option({
      type: optional(string),
      long: "update-url-base",
      description: "Base URL for update artifact downloads.",
    }),
    updateTarget,
    updateFormat,
    updateKeyEnv: releaseUpdateKeyEnv,
    githubRelease: flag({
      long: "github-release",
      description: "Create or update a GitHub release.",
      defaultValue: () => false,
    }),
    releaseTag: option({
      type: optional(string),
      long: "release-tag",
      description: "Git tag for the release.",
    }),
    releaseName: option({
      type: optional(string),
      long: "release-name",
      description: "Human-readable release name.",
    }),
    dryRun: flag({
      long: "dry-run",
      description: "Print the release plan without running release steps.",
      defaultValue: () => false,
    }),
  },
  handler: async (args) => {
    await runPackageRelease({
      root: args.path,
      version: args.version,
      mode: args.mode as ReleaseMode | undefined,
      signingConfig: args.signingConfig,
      signingPlatform: args.signingPlatform as SignPlatform | undefined,
      notarize: args.notarize,
      updateUrlBase: args.updateUrlBase,
      updateTarget: args.updateTarget,
      updateFormat: args.updateFormat as UpdatePackageFormat | undefined,
      updateKeyEnv: args.updateKeyEnv,
      githubRelease: args.githubRelease,
      releaseTag: args.releaseTag,
      releaseName: args.releaseName,
      dryRun: args.dryRun,
    });
  },
});

const packageCommands = subcommands({
  name: "package",
  description: "Package and release Cefari apps.",
  cmds: {
    package: packageApp,
    sign: packageSign,
    notarize: packageNotarize,
    update: packageUpdate,
    release: packageRelease,
  },
});

export const cefariCli = subcommands({
  name: "cefari",
  version: VERSION,
  description: "Develop, build, package, and release Cefari apps.",
  cmds: {
    dev,
    build,
    package: packageCommands,
  },
});

const packageSubcommands = new Set(["sign", "notarize", "update", "release", "package"]);
const helpArgs = new Set(["--help", "-h", "--version", "-v"]);

function normalizeArgv(argv: string[]): string[] {
  const packageIndex = argv[0] === "package" ? 0 : argv[2] === "package" ? 2 : -1;
  if (packageIndex === -1) {
    return argv;
  }

  const next = argv[packageIndex + 1];
  if (next === undefined || (!helpArgs.has(next) && (next.startsWith("-") || !packageSubcommands.has(next)))) {
    return [...argv.slice(0, packageIndex + 1), "package", ...argv.slice(packageIndex + 1)];
  }

  return argv;
}

export async function runCefariCli(argv: string[] = process.argv): Promise<void> {
  await run(binary(cefariCli), normalizeArgv(argv));
}
