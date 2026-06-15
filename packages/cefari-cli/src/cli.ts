import { binary, command, flag, number, option, optional, positional, run, string, subcommands } from "cmd-ts";
import { runCefariDev } from "./dev.js";

export const VERSION = "0.1.0";

export type CliTopLevelCommand = "init" | "dev" | "build" | "package" | "unknown";
export type CliPackageCommand = "package" | "sign" | "notarize" | "update" | "release";

type PlaceholderCommand =
  | "init"
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

const init = command({
  name: "init",
  description: "Create a new Cefari project.",
  args: {
    path: projectPath,
  },
  handler: () => placeholder("init"),
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
  handler: () => placeholder("build"),
});

const packageApp = command({
  name: "package",
  description: "Create a native package assembly.",
  args: {
    path: projectPath,
    release,
    releaseVersion,
  },
  handler: () => placeholder("package"),
});

const packageSign = command({
  name: "sign",
  description: "Sign packaged release artifacts.",
  args: {
    artifact,
  },
  handler: () => placeholder("package sign"),
});

const packageNotarize = command({
  name: "notarize",
  description: "Notarize packaged macOS artifacts.",
  args: {
    artifact,
  },
  handler: () => placeholder("package notarize"),
});

const packageUpdate = command({
  name: "update",
  description: "Create updater metadata for a release artifact.",
  args: {
    artifact,
    url,
    releaseVersion,
  },
  handler: () => placeholder("package update"),
});

const packageRelease = command({
  name: "release",
  description: "Run the release packaging pipeline.",
  args: {
    path: projectPath,
    releaseVersion,
  },
  handler: () => placeholder("package release"),
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
  description: "Create, develop, build, package, and release Cefari apps.",
  cmds: {
    init,
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
