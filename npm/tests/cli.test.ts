import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import test from "node:test";
import { createLogStore, toSentryLogRecord, type SentryLogRecord } from "../src/logs.js";
import { runLogsExportSentry, type LogExportSentrySink } from "../src/logs-cli.js";

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

test("documents Sentry log export command options", async () => {
  const { stdout } = await cefari(["logs", "export", "sentry", "--help"]);

  assert.match(stdout, /--dsn/);
  assert.match(stdout, /--environment/);
  assert.match(stdout, /--release/);
  assert.match(stdout, /--cursor/);
  assert.match(stdout, /--batch-size/);
  assert.match(stdout, /--level/);
  assert.match(stdout, /--scope/);
  assert.match(stdout, /--once/);
  assert.match(stdout, /--poll-ms/);
  assert.match(stdout, /--dry-run/);
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

test("dry-runs Sentry log export without sending or advancing the cursor", async () => {
  const databasePath = await seedLogDatabase();

  try {
    const { stdout } = await cefari([
      "logs",
      "export",
      "sentry",
      "--dry-run",
      "--level",
      "info",
      "--scope",
      "daemon",
    ], {
      env: { CEFARI_LOG_DATABASE: databasePath },
    });
    const records = JSON.parse(stdout) as Array<{
      level: string;
      message: string;
      timestamp: number;
      attributes: Record<string, unknown>;
    }>;

    assert.equal(Number.isFinite(records[0]?.timestamp), true);
    assert.deepEqual(records, [
      {
        level: "info",
        message: "daemon ready",
        timestamp: records[0]?.timestamp,
        attributes: {
          method: "start",
          "cefari.scope": "daemon",
          "cefari.log_id": 2,
          "cefari.pid": process.pid,
        },
      },
    ]);

    const store = createLogStore({ databasePath });
    try {
      assert.equal(store.exportCursor("sentry").lastExportedId, 0);
    } finally {
      store.close();
    }
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("Sentry log export advances the cursor after successful flush", async () => {
  const databasePath = await seedLogDatabase();
  const exported: string[] = [];

  try {
    await runLogsExportSentry({
      databasePath,
      dsn: "https://example@sentry.invalid/1",
      once: true,
      batchSize: 2,
      sinkFactory: () => fakeSink({
        exportMessages: exported,
      }),
    });

    assert.deepEqual(exported, ["debug hidden", "daemon ready"]);
    const store = createLogStore({ databasePath });
    try {
      assert.equal(store.exportCursor("sentry").lastExportedId, 2);
    } finally {
      store.close();
    }
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("Sentry log export sends all Cefari scopes through one path", async () => {
  const databasePath = await seedLogDatabase();
  const exported: SentryLogRecord[] = [];

  try {
    await runLogsExportSentry({
      databasePath,
      dsn: "https://example@sentry.invalid/1",
      once: true,
      cursor: "sentry-all-scopes",
      sinkFactory: () => fakeSink({
        exportRecords: exported,
      }),
    });

    assert.deepEqual(
      exported.map((record) => [
        record.level,
        record.message,
        record.attributes["cefari.scope"],
        record.attributes["cefari.log_id"],
      ]),
      [
        ["debug", "debug hidden", "app", 1],
        ["info", "daemon ready", "daemon", 2],
        ["warn", "worker warning", "worker:thumbnailer", 3],
        ["error", "runtime failed", "cefari", 4],
      ],
    );

    const store = createLogStore({ databasePath });
    try {
      assert.equal(store.exportCursor("sentry-all-scopes").lastExportedId, 4);
    } finally {
      store.close();
    }
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("Sentry log export does not advance the cursor after send failure", async () => {
  const databasePath = await seedLogDatabase();

  try {
    await assert.rejects(
      runLogsExportSentry({
        databasePath,
        dsn: "https://example@sentry.invalid/1",
        once: true,
        sinkFactory: () => fakeSink({
          failExport: true,
        }),
      }),
      /simulated Sentry send failure/,
    );

    const store = createLogStore({ databasePath });
    try {
      assert.equal(store.exportCursor("sentry").lastExportedId, 0);
    } finally {
      store.close();
    }
  } finally {
    await rm(dirname(databasePath), { recursive: true, force: true });
  }
});

test("Sentry log export filters by level and scope", async () => {
  const databasePath = await seedLogDatabase();
  const exported: string[] = [];

  try {
    await runLogsExportSentry({
      databasePath,
      dsn: "https://example@sentry.invalid/1",
      once: true,
      level: "warn",
      scope: "worker:thumbnailer",
      sinkFactory: () => fakeSink({
        exportMessages: exported,
      }),
    });

    assert.deepEqual(exported, ["worker warning"]);
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

function fakeSink(
  options: { exportMessages?: string[]; exportRecords?: SentryLogRecord[]; failExport?: boolean } = {},
): LogExportSentrySink {
  return {
    async export(records) {
      if (options.failExport === true) {
        throw new Error("simulated Sentry send failure");
      }
      options.exportMessages?.push(...records.map((record) => record.message));
      const sentryRecords = records.map(toSentryLogRecord);
      options.exportRecords?.push(...sentryRecords);
      return sentryRecords;
    },
    async flush() {
      return true;
    },
  };
}
