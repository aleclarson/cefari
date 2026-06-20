import { type BuildOptions, runCefariBuild } from "./build.js";
import { type DevOptions, runCefariDev } from "./dev.js";
import { type PackageOptions, runCefariPackage } from "./package.js";
import { currentPlatform } from "./platform.js";
import type { CefariBuildTarget } from "./platform.js";

export type CefariRuntimeTarget = "desktop" | "ios" | "android";

export type TargetedBuildOptions = Omit<BuildOptions, "target"> & {
  target?: CefariRuntimeTarget;
  desktopBuildTarget?: CefariBuildTarget;
};

export type TargetedDevOptions = DevOptions & {
  target?: CefariRuntimeTarget;
};

export type TargetedPackageOptions = PackageOptions & {
  target?: CefariRuntimeTarget;
};

export interface IosSimulatorPlan {
  prerequisites: string[];
  commands: string[][];
}

export async function runTargetedDev(
  options: TargetedDevOptions = {},
): Promise<void> {
  const target = options.target ?? "desktop";
  if (target === "desktop") {
    await runCefariDev(options);
    return;
  }
  if (target === "ios") {
    runIosSimulatorDev();
    return;
  }
  throw unsupportedTarget("dev", target);
}

export async function runTargetedBuild(
  options: TargetedBuildOptions = {},
): Promise<void> {
  const target = options.target ?? "desktop";
  if (target === "desktop") {
    await runCefariBuild({
      root: options.root,
      release: options.release,
      target: options.desktopBuildTarget,
    });
    return;
  }
  throw unsupportedTarget("build", target);
}

export async function runTargetedPackage(
  options: TargetedPackageOptions = {},
): Promise<void> {
  const target = options.target ?? "desktop";
  if (target === "desktop") {
    await runCefariPackage(options);
    return;
  }
  throw unsupportedTarget("package", target);
}

function unsupportedTarget(
  command: string,
  target: Exclude<CefariRuntimeTarget, "desktop">,
): Error {
  return new Error(
    `cefari ${command} --target ${target} is recognized, but ${target} runtime support is not implemented yet`,
  );
}

export function iosSimulatorPlan(): IosSimulatorPlan {
  return {
    prerequisites: [
      "Xcode or Xcode Command Line Tools installed",
      "an available iOS simulator runtime",
      "a generated Cefari iOS host app bundle",
    ],
    commands: [
      ["xcrun", "simctl", "boot", "<device>"],
      ["xcrun", "simctl", "install", "booted", "<cefari-ios-app-bundle>"],
      ["xcrun", "simctl", "launch", "booted", "<bundle-id>"],
    ],
  };
}

function runIosSimulatorDev(): never {
  ensureCommand(
    "xcode-select",
    ["-p"],
    "Xcode command line tools are required for `cefari dev --target ios`.",
  );
  ensureCommand(
    "xcrun",
    ["simctl", "help"],
    "`xcrun simctl` is required for `cefari dev --target ios`.",
  );

  const plan = iosSimulatorPlan();
  throw new Error(
    [
      "`cefari dev --target ios` reached simulator tooling, but Cefari does not generate the Swift-owned iOS host app bundle yet.",
      "Planned simulator launch commands:",
      ...plan.commands.map((command) => `  ${command.join(" ")}`),
    ].join("\n"),
  );
}

function ensureCommand(command: string, args: string[], message: string): void {
  const result = currentPlatform().spawnSync(command, args, {
    stdio: "ignore",
    shell: process.platform === "win32",
  });
  if (result.error !== undefined || result.status !== 0) {
    throw new Error(message);
  }
}
