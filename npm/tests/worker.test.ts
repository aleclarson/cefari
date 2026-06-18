import assert from "node:assert/strict";
import test from "node:test";
import {
  defineWorker,
  runCefariWorker,
  type InferCefariWorker,
  type WorkerInput,
  type WorkerMessage,
  type WorkerOutput,
  type WorkerProtocolIo,
} from "../src/worker.js";

test("runs a worker from a start envelope and writes protocol lines", async () => {
  const stdout: string[] = [];
  const worker = defineWorker<{ value: number }, { doubled: number }, { seen: number }>({
    async run(input, context) {
      await context.postMessage({ seen: input.value });
      return { doubled: input.value * 2 };
    },
  });

  const exitCode = await runCefariWorker(worker, protocolIo({
    type: "start",
    id: "worker-1",
    input: { value: 21 },
  }, stdout));

  assert.equal(exitCode, 0);
  assert.deepEqual(stdout.map((line) => JSON.parse(line)), [
    {
      type: "message",
      id: "worker-1",
      payload: { seen: 21 },
    },
    {
      type: "result",
      id: "worker-1",
      payload: { doubled: 42 },
    },
  ]);
});

test("writes protocol errors for malformed input", async () => {
  const stdout: string[] = [];
  const worker = defineWorker({
    run() {
      return null;
    },
  });

  const exitCode = await runCefariWorker(worker, {
    readStdin: async () => "not json",
    writeStdout(line) {
      stdout.push(line.trimEnd());
    },
  });

  assert.equal(exitCode, 1);
  assert.equal(stdout.length, 1);
  const errorEnvelope = JSON.parse(stdout[0]);
  assert.equal(errorEnvelope.type, "error");
  assert.equal(errorEnvelope.id, null);
  assert.match(errorEnvelope.error.message, /worker protocol input must be JSON/);
  assert.deepEqual(Object.keys(errorEnvelope), ["type", "id", "error"]);
  assert.deepEqual(Object.keys(errorEnvelope.error), ["message"]);
});

test("writes protocol errors for invalid start envelopes", async () => {
  const stdout: string[] = [];
  const worker = defineWorker({
    run() {
      return null;
    },
  });

  const exitCode = await runCefariWorker(worker, protocolIo({ type: "start", id: "" }, stdout));

  assert.equal(exitCode, 1);
  assert.deepEqual(JSON.parse(stdout[0]), {
    type: "error",
    id: null,
    error: {
      message: "worker protocol input id must be a non-empty string",
    },
  });
});

test("exposes worker type helpers", () => {
  const worker = defineWorker<{ imageId: string }, { outputPath: string }, { progress: number }>({
    run(input, context) {
      void context.postMessage({ progress: 1 });
      return { outputPath: input.imageId };
    },
  });

  type Contract = InferCefariWorker<typeof worker>;
  const input: WorkerInput<typeof worker> = { imageId: "abc" };
  const output: WorkerOutput<typeof worker> = { outputPath: "cache/abc.png" };
  const message: WorkerMessage<typeof worker> = { progress: 0.5 };
  const contract: Contract = { input, output, message };

  assert.deepEqual(contract, {
    input: { imageId: "abc" },
    output: { outputPath: "cache/abc.png" },
    message: { progress: 0.5 },
  });
});

function protocolIo(input: unknown, stdout: string[]): WorkerProtocolIo {
  return {
    readStdin: async () => JSON.stringify(input),
    writeStdout(line) {
      stdout.push(line.trimEnd());
    },
  };
}
