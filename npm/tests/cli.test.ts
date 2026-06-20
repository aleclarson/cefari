import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import test from "node:test";
import { createLogStore } from "../src/logs.js";

const execFileAsync = promisify(execFile);
const testDir = dirname(fileURLToPath(import.meta.url));
const cliPath = resolve(testDir, "../bin/cefari.js");

async function cefari(args: string[], options: { env?: NodeJS.ProcessEnv } = {}) {
  return execFileAsync("deno", [
    "run",
    "-A",
    cliPath,
    ...args,
  ], {
    cwd: resolve(testDir, ".."),
    env: {
      ...process.env,
      ...options.env,
    },
  });
}

test("prints root help with the simplified command set", async () => {
  const { stdout } = await cefari(["--help"]);

  assert.doesNotMatch(stdout, /init/);
  assert.match(stdout, /dev/);
  assert.match(stdout, /build/);
  assert.match(stdout, /package/);
  assert.match(stdout, /logs/);
  assert.doesNotMatch(stdout, /codesign/);
  assert.doesNotMatch(stdout, /make-update/);
  assert.doesNotMatch(stdout, /doctor/);
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
  assert.match(stdout, /--target/);
  assert.equal(stdout.includes(`--${"frontend"}-port`), false);
});

test("documents build target flag", async () => {
  const { stdout } = await cefari(["build", "--help"]);

  assert.match(stdout, /--target/);
  assert.match(stdout, /ios/);
  assert.match(stdout, /windows-x64/);
});

test("recognizes unsupported mobile runtime targets", async () => {
  await assert.rejects(
    cefari(["dev", "--target", "android"]),
    /android runtime support is not implemented yet/,
  );
  await assert.rejects(
    cefari(["build", "--target", "ios"]),
    /ios runtime support is not implemented yet/,
  );
  await assert.rejects(
    cefari(["package", "--target", "android"]),
    /android runtime support is not implemented yet/,
  );
});

test("documents nested package release subcommand", async () => {
  const { stdout } = await cefari(["package", "release", "--help"]);

  assert.match(stdout, /--version/);
  assert.match(stdout, /--update-key-env/);
  assert.match(stdout, /--dry-run/);
});

test("documents bare package command options", async () => {
  const { stdout } = await cefari(["package", "package", "--help"]);

  assert.match(stdout, /--release-version/);
});

test("does not expose Sentry log export as a logs subcommand", async () => {
  const { stdout } = await cefari(["logs", "--help"]);

  assert.doesNotMatch(stdout, /export/);
  await assert.rejects(
    cefari(["logs", "export", "sentry"]),
    (error) => {
      assert.ok(error instanceof Error);
      assert.match(error.message, /Not a valid subcommand name/);
      return true;
    },
  );
});

test("prints canonical log database path", async () => {
  const databasePath = await testLogDatabasePath();

  try {
    const { stdout } = await cefari(["logs", "path"], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });

    assert.equal(stdout.trim(), databasePath);
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("pages log rows with filters and hides debug rows by default", async () => {
  const databasePath = await seedLogDatabase();

  try {
    const defaultPage = await cefari(["logs", "page", "--json"], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });
    const defaultRows = JSON.parse(defaultPage.stdout) as Array<{ message: string }>;
    assert.deepEqual(defaultRows.map((row) => row.message), ["daemon ready", "worker warning", "runtime failed"]);

    const allScopesPage = await cefari(["logs", "page", "--json", "--level", "debug", "--limit", "10"], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });
    const allScopesRows = JSON.parse(allScopesPage.stdout) as Array<{ scope: string }>;
    assert.deepEqual(
      new Set(allScopesRows.map((row) => row.scope)),
      new Set(["app", "daemon", "worker:thumbnailer", "cefari"]),
    );

    const filteredPage = await cefari([
      "logs",
      "page",
      "--json",
      "--level",
      "debug",
      "--scope",
      "daemon",
      "--property",
      "method=start",
      "--grep",
      "ready",
    ], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });
    const filteredRows = JSON.parse(filteredPage.stdout) as Array<{ scope: string; message: string }>;
    assert.deepEqual(
      filteredRows.map((row) => ({ scope: row.scope, message: row.message })),
      [{ scope: "daemon", message: "daemon ready" }],
    );
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("tails log rows with one-shot polling", async () => {
  const databasePath = await seedLogDatabase();

  try {
    const { stdout } = await cefari(["logs", "tail", "--once", "--scope", "worker:thumbnailer"], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });

    assert.match(stdout, /worker:thumbnailer warn worker warning/);
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("expands collapsed log values", async () => {
  const databasePath = await testLogDatabasePath();
  const store = createLogStore({ databasePath, inlineByteLimit: 4 });
  const entry = store.append({
    scope: "app",
    level: "info",
    message: "payload",
    properties: { body: "long payload" },
  });
  store.close();

  try {
    const id = entry.properties.body as string;
    const { stdout } = await cefari(["logs", "expand", id], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });
    const expanded = JSON.parse(stdout) as { body: unknown };

    assert.equal(expanded.body, "long payload");
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

async function testLogDatabasePath(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "cefari-cli-logs-"));
  return join(root, "cefari.sqlite");
}

async function seedLogDatabase(): Promise<string> {
  const databasePath = await testLogDatabasePath();
  const store = createLogStore({ databasePath });
  store.append({
    scope: "app",
    level: "debug",
    message: "debug hidden",
    properties: { method: "debug" },
  });
  store.append({
    scope: "daemon",
    level: "info",
    message: "daemon ready",
    properties: { method: "start" },
  });
  store.append({
    scope: "worker:thumbnailer",
    level: "warn",
    message: "worker warning",
    properties: { worker: "thumbnailer" },
  });
  store.append({
    scope: "cefari",
    level: "error",
    message: "runtime failed",
    properties: { target: "runtime" },
  });
  store.close();
  return databasePath;
}
