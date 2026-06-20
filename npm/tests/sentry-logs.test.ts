import assert from "node:assert/strict";
import { test } from "node:test";

import {
  createSentryLogSink,
  type SentryLogClient,
  type SentryLogLogger,
  type SentryLogSinkOptions,
} from "../src/sentry-logs.js";

type SentLog = {
  level: keyof SentryLogLogger;
  message: string;
  attributes: Record<string, unknown> | undefined;
};

test("createSentryLogSink initializes Sentry logs with downstream options", async () => {
  const sent: SentLog[] = [];
  const initOptions: unknown[] = [];
  const client: SentryLogClient = {
    init(options) {
      initOptions.push(options);
    },
    logger: createFakeLogger(sent),
    async flush(timeout) {
      assert.equal(timeout, 250);
      return true;
    },
  };

  const beforeSendLog: SentryLogSinkOptions["beforeSendLog"] = (log) => log;
  const sink = createSentryLogSink({
    client,
    dsn: "https://example@sentry.invalid/1",
    environment: "test",
    release: "cefari@0.1.0",
    sampleRate: 0.5,
    beforeSendLog,
  });

  const exported = await sink.export([
    {
      id: 42,
      at: "2026-01-02T03:04:05.000Z",
      scope: "worker:thumbnailer",
      level: "log",
      pid: 1234,
      message: "thumbnail.ready",
      properties: {
        durationMs: 17,
        file: "image.png",
      },
    },
  ]);

  assert.deepEqual(initOptions, [
    {
      dsn: "https://example@sentry.invalid/1",
      environment: "test",
      release: "cefari@0.1.0",
      sampleRate: 0.5,
      beforeSendLog,
      enableLogs: true,
    },
  ]);
  assert.deepEqual(sent, [
    {
      level: "info",
      message: "thumbnail.ready",
      attributes: {
        durationMs: 17,
        file: "image.png",
        "cefari.scope": "worker:thumbnailer",
        "cefari.log_id": 42,
        "cefari.pid": 1234,
      },
    },
  ]);
  assert.deepEqual(exported, [
    {
      level: "info",
      message: "thumbnail.ready",
      timestamp: Date.parse("2026-01-02T03:04:05.000Z"),
      attributes: {
        durationMs: 17,
        file: "image.png",
        "cefari.scope": "worker:thumbnailer",
        "cefari.log_id": 42,
        "cefari.pid": 1234,
      },
    },
  ]);
  assert.equal(await sink.flush(250), true);
});

test("createSentryLogSink preserves scopes and maps Cefari levels to Sentry levels", async () => {
  const sent: SentLog[] = [];
  const client: SentryLogClient = {
    init() {},
    logger: createFakeLogger(sent),
  };
  const sink = createSentryLogSink({
    client,
    dsn: "https://example@sentry.invalid/1",
  });

  await sink.export([
    baseRecord({ id: 1, scope: "cefari", level: "debug" }),
    baseRecord({ id: 2, scope: "app", level: "info" }),
    baseRecord({ id: 3, scope: "daemon", level: "warn" }),
    baseRecord({ id: 4, scope: "worker:jobs", level: "error" }),
  ]);

  assert.deepEqual(
    sent.map((log) => [log.level, log.attributes?.["cefari.scope"]]),
    [
      ["debug", "cefari"],
      ["info", "app"],
      ["warn", "daemon"],
      ["error", "worker:jobs"],
    ],
  );
});

function createFakeLogger(sent: SentLog[]): SentryLogLogger {
  return {
    debug: (message, attributes) => sent.push({ level: "debug", message, attributes }),
    error: (message, attributes) => sent.push({ level: "error", message, attributes }),
    info: (message, attributes) => sent.push({ level: "info", message, attributes }),
    warn: (message, attributes) => sent.push({ level: "warn", message, attributes }),
  };
}

function baseRecord(input: {
  id: number;
  scope: "app" | "cefari" | "daemon" | `worker:${string}`;
  level: "debug" | "error" | "info" | "warn";
}) {
  return {
    id: input.id,
    at: "2026-01-02T03:04:05.000Z",
    scope: input.scope,
    level: input.level,
    pid: 1234,
    message: `message.${input.id}`,
    properties: {
      traceId: `trace-${input.id}`,
    },
  };
}
