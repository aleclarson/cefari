#!/usr/bin/env node

"use strict";

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

const platformPackages = {
  "darwin:arm64": "@cefari/cli-darwin-arm64",
  "darwin:x64": "@cefari/cli-darwin-x64",
  "linux:x64": "@cefari/cli-linux-x64",
  "win32:x64": "@cefari/cli-win32-x64",
};

const platformKey = `${process.platform}:${process.arch}`;
const packageName = platformPackages[platformKey];

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (!packageName) {
  fail(`Unsupported platform for @cefari/cli: ${platformKey}`);
}

let packageJsonPath;
try {
  packageJsonPath = require.resolve(`${packageName}/package.json`);
} catch (error) {
  fail(
    `Missing ${packageName}. Reinstall @cefari/cli for ${platformKey} or install ${packageName} explicitly.`
  );
}

const binaryName = process.platform === "win32" ? "cefari.exe" : "cefari";
const binaryPath = join(packageJsonPath, "..", "bin", binaryName);

if (!existsSync(binaryPath)) {
  fail(`Missing Cefari binary at ${binaryPath}`);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false,
});

if (result.error) {
  fail(`Failed to run Cefari binary: ${result.error.message}`);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
} else {
  process.exit(result.status === null ? 1 : result.status);
}
