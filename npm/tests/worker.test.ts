import assert from "node:assert/strict";
import test from "node:test";
import {
  defineWorker,
  getWorkerResources,
  runCefariWorker,
  workerNativePath,
  type InferCefariWorker,
  type WorkerInit,
  type WorkerMethods,
  type WorkerProtocolIo,
} from "../src/worker.js";

test("runs worker methods from request envelopes and writes protocol lines", async () => {
  const stdout: string[] = [];
  const worker = defineWorker((init: { multiplier: number }) => ({
    async double(input: { value: number }, context) {
      await context.postMessage({ seen: input.value });
      return { doubled: input.value * init.multiplier };
    },
  }));

  const exitCode = await runCefariWorker(worker, protocolIo([
    {
      type: "start",
      id: "worker-1",
      input: { multiplier: 2 },
    },
    {
      type: "request",
      requestId: "request-1",
      method: "double",
      input: { value: 21 },
    },
  ], stdout));

  assert.equal(exitCode, 0);
  assert.deepEqual(stdout.map((line) => JSON.parse(line)), [
    {
      type: "message",
      id: "worker-1",
      requestId: "request-1",
      method: "double",
      payload: { seen: 21 },
    },
    {
      type: "result",
      id: "worker-1",
      requestId: "request-1",
      method: "double",
      payload: { doubled: 42 },
    },
  ]);
});

test("exposes worker resources from the start envelope", async () => {
  const stdout: string[] = [];
  let nativePath = "";
  let resourceDir = "";
  const worker = defineWorker(() => {
    nativePath = workerNativePath("bin/thumb");
    resourceDir = getWorkerResources().resourceDir;
    return {
      paths() {
        return { nativePath, resourceDir };
      },
    };
  });

  const exitCode = await runCefariWorker(worker, protocolIo([
    {
      type: "start",
      id: "worker-1",
      input: null,
      resources: {
        id: "thumbnailer",
        resourceDir: "/app/resources",
        nativeDir: "/app/resources/workers/thumbnailer/native",
        native: {
          "bin/thumb": "/app/resources/workers/thumbnailer/native/bin/thumb",
        },
      },
    },
    {
      type: "request",
      requestId: "request-1",
      method: "paths",
      input: null,
    },
  ], stdout));

  assert.equal(exitCode, 0);
  assert.deepEqual(JSON.parse(stdout[0]), {
    type: "result",
    id: "worker-1",
    requestId: "request-1",
    method: "paths",
    payload: {
      nativePath: "/app/resources/workers/thumbnailer/native/bin/thumb",
      resourceDir: "/app/resources",
    },
  });
});

test("reports unconfigured worker native payload paths", async () => {
  const worker = defineWorker(() => {
    workerNativePath("bin/missing");
    return {};
  });
  const stdout: string[] = [];

  const exitCode = await runCefariWorker(worker, protocolIo([
    {
      type: "start",
      id: "worker-1",
      input: null,
      resources: {
        id: "thumbnailer",
        resourceDir: "/app/resources",
        nativeDir: "/app/resources/workers/thumbnailer/native",
        native: {},
      },
    },
  ], stdout));

  assert.equal(exitCode, 1);
  assert.match(JSON.parse(stdout[0]).error.message, /worker native payload "bin\/missing" is not configured/);
});

test("writes protocol errors for malformed input", async () => {
  const stdout: string[] = [];
  const worker = defineWorker(() => ({}));

  const exitCode = await runCefariWorker(worker, {
    readLine: async () => "not json",
    writeStdout(line) {
      stdout.push(line.trimEnd());
    },
  });

  assert.equal(exitCode, 1);
  assert.equal(stdout.length, 1);
  const errorEnvelope = JSON.parse(stdout[0]);
  assert.equal(errorEnvelope.type, "error");
  assert.equal(errorEnvelope.id, null);
  assert.match(errorEnvelope.error.message, /worker protocol start must be JSON/);
  assert.deepEqual(Object.keys(errorEnvelope), ["type", "id", "requestId", "method", "error"]);
  assert.deepEqual(Object.keys(errorEnvelope.error), ["message"]);
});

test("writes protocol errors for invalid start envelopes", async () => {
  const stdout: string[] = [];
  const worker = defineWorker(() => ({}));

  const exitCode = await runCefariWorker(worker, protocolIo([{ type: "start", id: "" }], stdout));

  assert.equal(exitCode, 1);
  assert.deepEqual(JSON.parse(stdout[0]), {
    type: "error",
    id: null,
    requestId: null,
    method: null,
    error: {
      message: "worker protocol start id must be a non-empty string",
    },
  });
});

test("writes protocol errors for invalid method requests without exiting", async () => {
  const stdout: string[] = [];
  const worker = defineWorker(() => ({
    ok() {
      return { ok: true };
    },
  }));

  const exitCode = await runCefariWorker(worker, protocolIo([
    { type: "start", id: "worker-1", input: null },
    { type: "request", requestId: "request-1", method: "missing", input: null },
    { type: "request", requestId: "request-2", method: "ok", input: null },
  ], stdout));

  assert.equal(exitCode, 0);
  assert.deepEqual(stdout.map((line) => JSON.parse(line)), [
    {
      type: "error",
      id: "worker-1",
      requestId: "request-1",
      method: "missing",
      error: {
        message: 'worker method "missing" is not defined',
      },
    },
    {
      type: "result",
      id: "worker-1",
      requestId: "request-2",
      method: "ok",
      payload: { ok: true },
    },
  ]);
});

test("exposes worker type helpers", () => {
  const worker = defineWorker((init: { cacheDir: string }) => ({
    render(input: { imageId: string }, context) {
      void context.postMessage({ progress: 1 });
      return { outputPath: `${init.cacheDir}/${input.imageId}.png` };
    },
  }));

  type Contract = InferCefariWorker<typeof worker>;
  const init: WorkerInit<typeof worker> = { cacheDir: "cache" };
  const output: WorkerMethods<typeof worker>["render"]["output"] = { outputPath: "cache/abc.png" };
  const message: WorkerMethods<typeof worker>["render"]["message"] = { progress: 0.5 };
  const contract: Contract = {
    init,
    methods: {
      render: {
        input: { imageId: "abc" },
        output,
        message,
      },
    },
  };

  assert.deepEqual(contract, {
    init: { cacheDir: "cache" },
    methods: {
      render: {
        input: { imageId: "abc" },
        output: { outputPath: "cache/abc.png" },
        message: { progress: 0.5 },
      },
    },
  });
});

function protocolIo(input: unknown[], stdout: string[]): WorkerProtocolIo {
  const lines = input.map((line) => JSON.stringify(line));
  return {
    readLine: async () => lines.shift() ?? null,
    writeStdout(line) {
      stdout.push(line.trimEnd());
    },
  };
}
