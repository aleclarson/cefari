import { type BuildOptions, runCefariBuild } from "./build.js";
import { type DevOptions, runCefariDev } from "./dev.js";
import { type PackageOptions, runCefariPackage } from "./package.js";
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

export async function runTargetedDev(
  options: TargetedDevOptions = {},
): Promise<void> {
  const target = options.target ?? "desktop";
  if (target === "desktop") {
    await runCefariDev(options);
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
