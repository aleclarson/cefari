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

test("documents Vite dev port flag", async () => {
  const { stdout } = await cefari(["dev", "--help"]);

  assert.match(stdout, /--vite-port/);
  assert.doesNotMatch(stdout, /--frontend-port/);
});

test("documents nested package release subcommand", async () => {
  const { stdout } = await cefari(["package", "release", "--help"]);

  assert.match(stdout, /--version/);
  assert.match(stdout, /--dry-run/);
});

test("documents bare package command options", async () => {
  const { stdout } = await cefari(["package", "package", "--help"]);

  assert.match(stdout, /--release-version/);
});
