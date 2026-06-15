import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { promisify } from "node:util";
import test from "node:test";

const execFileAsync = promisify(execFile);
const testDir = dirname(fileURLToPath(import.meta.url));
const cliPath = resolve(testDir, "../bin/cefari.js");

async function cefari(args: string[]) {
  return execFileAsync(process.execPath, [cliPath, ...args], {
    cwd: resolve(testDir, ".."),
  });
}

test("prints root help with the simplified command set", async () => {
  const { stdout } = await cefari(["--help"]);

  assert.match(stdout, /init/);
  assert.match(stdout, /dev/);
  assert.match(stdout, /build/);
  assert.match(stdout, /package/);
  assert.doesNotMatch(stdout, /codesign/);
  assert.doesNotMatch(stdout, /make-update/);
  assert.doesNotMatch(stdout, /doctor/);
  assert.doesNotMatch(stdout, /logs/);
  assert.doesNotMatch(stdout, /info/);
  assert.doesNotMatch(stdout, /clean/);
});

test("prints package help with release-management subcommands", async () => {
  const { stdout } = await cefari(["package", "--help"]);

  assert.match(stdout, /sign/);
  assert.match(stdout, /notarize/);
  assert.match(stdout, /update/);
  assert.match(stdout, /release/);
});

test("prints version", async () => {
  const { stdout } = await cefari(["--version"]);

  assert.equal(stdout.trim(), "0.1.0");
});

test("routes nested package subcommands", async () => {
  const { stdout } = await cefari(["package", "release"]);

  assert.equal(stdout.trim(), "package release is not implemented yet.");
});

test("routes bare package command to package assembly", async () => {
  const { stdout } = await cefari(["package"]);

  assert.equal(stdout.trim(), "package is not implemented yet.");
});
