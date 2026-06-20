import {
  createDebug,
  createLogger,
  createLogStore,
  formatLogEntry,
  subtractHours,
  toLogExportRecord,
  toSentryLogLevel,
  toSentryLogRecord,
  type LogEntry,
} from "../../src/logs.ts";

function assert(condition: unknown, message?: string): asserts condition {
  if (!condition) {
    throw new Error(message ?? "assertion failed");
  }
}

function assertEquals(actual: unknown, expected: unknown): void {
  const actualJson = JSON.stringify(sortJson(actual));
  const expectedJson = JSON.stringify(sortJson(expected));
  if (actualJson !== expectedJson) {
    throw new Error(`expected ${expectedJson}, got ${actualJson}`);
  }
}

function sortJson(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortJson);
  }
  if (value === null || typeof value !== "object") {
    return value;
  }
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, nested]) => [key, sortJson(nested)]),
  );
}

async function createTestStore(options: { inlineByteLimit?: number } = {}) {
  const testDir = await Deno.makeTempDir({ prefix: "cefari-logs-test-" });
  const store = createLogStore({
    databasePath: `${testDir}/logs.sqlite`,
    inlineByteLimit: options.inlineByteLimit,
  });
  return {
    store,
    async cleanup() {
      store.close();
      await Deno.remove(testDir, { recursive: true });
    },
  };
}

Deno.test("logger writes compact log rows", async () => {
  const { store, cleanup } = await createTestStore();
  try {
    const logger = createLogger({ scope: "daemon", store, pid: 123 });

    logger.info("daemon.startup", {
      component: "daemon",
      port: 53117,
      nested: { ok: true },
    });

    const [entry] = store.query();
    assert(entry !== undefined);
    assertEquals(
      {
        ...entry,
        id: 1,
        at: "<date>",
      },
      {
        id: 1,
        at: "<date>",
        scope: "daemon",
        level: "info",
        pid: 123,
        message: "daemon.startup",
        properties: {
          component: "daemon",
          port: 53117,
          nested: { ok: true },
        },
      },
    );
  } finally {
    await cleanup();
  }
});

Deno.test("collapses long strings and large nested values", async () => {
  const { store, cleanup } = await createTestStore({ inlineByteLimit: 24 });
  try {
    const row = store.append({
      scope: "daemon",
      level: "info",
      message: "ipc.response_sent",
      properties: {
        body: "x".repeat(40),
        items: ["one", "two", "three", "four"],
        object: { text: "y".repeat(40) },
      },
    });

    assert(typeof row.properties.body === "string");
    assert(/^str_[0-9A-HJKMNP-TV-Z]{26}$/.test(row.properties.body));
    assert(typeof row.properties.items === "string");
    assert(/^arr_[0-9A-HJKMNP-TV-Z]{26}$/.test(row.properties.items));
    assert(typeof row.properties.object === "string");
    assert(/^obj_[0-9A-HJKMNP-TV-Z]{26}$/.test(row.properties.object));
    assertEquals(store.expand(row.properties.body)?.body, "x".repeat(40));
    assertEquals(store.expand(row.properties.items)?.body, ["one", "two", "three", "four"]);
    assertEquals(store.expand(row.properties.object)?.body, { text: "y".repeat(40) });
  } finally {
    await cleanup();
  }
});

Deno.test("redacts secrets before persistence", async () => {
  const { store, cleanup } = await createTestStore();
  try {
    store.append({
      scope: "app",
      level: "info",
      message: "auth.request",
      properties: {
        token: "secret",
        nested: {
          authorization: "Bearer secret",
        },
        env: {
          OPENAI_API_KEY: "secret",
          PATH: "/bin",
        },
      },
    });

    assertEquals(store.query()[0]?.properties, {
      token: "[redacted]",
      nested: {
        authorization: "[redacted]",
      },
      env: {
        OPENAI_API_KEY: "[redacted]",
        PATH: "/bin",
      },
    });
  } finally {
    await cleanup();
  }
});

Deno.test("queries by scope, grep, cursor, regex, and property", async () => {
  const { store, cleanup } = await createTestStore();
  try {
    store.append({ scope: "daemon", level: "info", message: "first", properties: { method: "a" } });
    store.append({ scope: "app", level: "info", message: "needle", properties: { method: "b" } });
    store.append({
      scope: "daemon",
      level: "info",
      message: "third",
      properties: { method: "b", durationMs: 38 },
    });

    assertEquals(store.query({ scope: "daemon" }).map((entry) => entry.message), ["first", "third"]);
    assertEquals(store.query({ grep: "needle" }).map((entry) => entry.message), ["needle"]);
    assertEquals(store.query({ properties: { method: "b" } }).map((entry) => entry.message), ["needle", "third"]);
    assertEquals(store.query({ properties: { durationMs: "38" } }).map((entry) => entry.message), ["third"]);
    assertEquals(store.query({ regex: "nee(dle)?|third" }).map((entry) => entry.message), ["needle", "third"]);
    assertEquals(store.query({ afterId: 1 }).map((entry) => entry.message), ["needle", "third"]);
    assertEquals(store.query({ beforeId: 3 }).map((entry) => entry.message), ["first", "needle"]);
  } finally {
    await cleanup();
  }
});

Deno.test("queries by minimum level and debug scope prefix", async () => {
  const { store, cleanup } = await createTestStore();
  try {
    const sessionDebug = createDebug("session.history", { scope: "cefari", store, pid: 123 });
    const configDebug = createDebug("config.reload", { scope: "cefari", store, pid: 123 });

    sessionDebug("history.normalized", { sessionId: "ses_1" });
    configDebug("config.refreshed");
    store.append({ scope: "cefari", level: "info", message: "runtime.ready" });
    store.append({ scope: "cefari", level: "warn", message: "runtime.slow" });

    assertEquals(store.query({ level: "info" }).map((entry) => entry.message), ["runtime.ready", "runtime.slow"]);
    assertEquals(store.query({ level: "debug" }).map((entry) => entry.message), [
      "history.normalized",
      "config.refreshed",
      "runtime.ready",
      "runtime.slow",
    ]);
    assertEquals(store.query({ debugScope: "session" }).map((entry) => entry.message), ["history.normalized"]);
    assertEquals(store.query({ debugScope: "session" })[0]?.properties, {
      debugScope: "session.history",
      sessionId: "ses_1",
    });
  } finally {
    await cleanup();
  }
});

Deno.test("retention removes old rows and unreferenced collapsed values", async () => {
  const { store, cleanup } = await createTestStore({ inlineByteLimit: 8 });
  try {
    const old = store.append({
      at: subtractHours(new Date(), 25),
      scope: "daemon",
      level: "info",
      message: "old",
      properties: { payload: "old value that collapses" },
    });
    const recent = store.append({
      scope: "daemon",
      level: "info",
      message: "recent",
      properties: { payload: "recent value that collapses" },
    });

    store.retainSince(subtractHours(new Date(), 24));

    assertEquals(store.query().map((entry) => entry.message), ["recent"]);
    assertEquals(store.expand(old.properties.payload as string), null);
    assertEquals(store.expand(recent.properties.payload as string)?.body, "recent value that collapses");
  } finally {
    await cleanup();
  }
});

Deno.test("formats log entries as timeline fields, message, and properties", () => {
  const entry: LogEntry = {
    id: 1,
    at: "2026-06-16T12:00:00.000Z",
    scope: "cefari",
    level: "info",
    pid: 123,
    message: "ipc.response_sent",
    properties: {
      component: "ipc",
      durationMs: 38,
      response: "obj_01K00000000000000000000000",
    },
  };

  assertEquals(
    formatLogEntry(entry),
    "1 2026-06-16T12:00:00.000Z cefari info ipc.response_sent pid=123 component=ipc durationMs=38 response={obj_01K00000000000000000000000}",
  );
});

Deno.test("maps local rows to vendor-neutral export records", () => {
  const entries: LogEntry[] = [
    {
      id: 1,
      at: "2026-06-16T12:00:00.000Z",
      scope: "cefari",
      level: "info",
      pid: 100,
      message: "runtime.ready",
      properties: { target: "runtime" },
    },
    {
      id: 2,
      at: "2026-06-16T12:00:01.000Z",
      scope: "app",
      level: "warn",
      pid: 101,
      message: "app.warning",
      properties: { windowId: "main" },
    },
    {
      id: 3,
      at: "2026-06-16T12:00:02.000Z",
      scope: "daemon",
      level: "error",
      pid: 102,
      message: "daemon.failed",
      properties: { connectionId: 7 },
    },
    {
      id: 4,
      at: "2026-06-16T12:00:03.000Z",
      scope: "worker:thumbnailer",
      level: "debug",
      pid: 103,
      message: "worker.started",
      properties: { worker: "thumbnailer", workerId: "thumbnailer-1" },
    },
  ];

  const records = entries.map(toLogExportRecord);

  assertEquals(records.map((record) => record.scope), [
    "cefari",
    "app",
    "daemon",
    "worker:thumbnailer",
  ]);
  assertEquals(records[0]?.attributes, {
    target: "runtime",
    "cefari.scope": "cefari",
    "cefari.log_id": 1,
    "cefari.pid": 100,
  });
  assertEquals(records[3]?.attributes, {
    worker: "thumbnailer",
    workerId: "thumbnailer-1",
    "cefari.scope": "worker:thumbnailer",
    "cefari.log_id": 4,
    "cefari.pid": 103,
  });
});

Deno.test("maps Cefari rows to Sentry-shaped log records", () => {
  assertEquals(toSentryLogLevel("debug"), "debug");
  assertEquals(toSentryLogLevel("info"), "info");
  assertEquals(toSentryLogLevel("log"), "info");
  assertEquals(toSentryLogLevel("warn"), "warn");
  assertEquals(toSentryLogLevel("error"), "error");

  const sentry = toSentryLogRecord({
    id: 5,
    at: "2026-06-16T12:00:04.000Z",
    scope: "daemon",
    level: "log",
    pid: 123,
    message: "daemon stdout remains protocol-owned",
    properties: { stream: "stderr" },
  });

  assertEquals(sentry, {
    level: "info",
    message: "daemon stdout remains protocol-owned",
    timestamp: Date.parse("2026-06-16T12:00:04.000Z"),
    attributes: {
      stream: "stderr",
      "cefari.scope": "daemon",
      "cefari.log_id": 5,
      "cefari.pid": 123,
    },
  });
});
