export interface CefariWorkerContext<Message = unknown> {
  postMessage(message: Message): Promise<void>;
}

export type CefariWorkerMethod<Input = unknown, Output = unknown, Message = unknown> = (
  input: Input,
  context: CefariWorkerContext<Message>,
) => Output | Promise<Output>;

type AnyCefariWorkerMethod = (
  input: any,
  context: CefariWorkerContext<any>,
) => any;

export type CefariWorkerMethods = Record<string, AnyCefariWorkerMethod>;

export type CefariWorkerFactory<Init = unknown, Methods extends CefariWorkerMethods = CefariWorkerMethods> = (
  init: Init,
) => Methods | Promise<Methods>;

export interface CefariWorkerMethodContract<Input = unknown, Output = unknown, Message = unknown> {
  input: Input;
  output: Output;
  message: Message;
}

export interface CefariWorkerContract<Init = unknown, Methods extends Record<string, CefariWorkerMethodContract> = Record<string, CefariWorkerMethodContract>> {
  init: Init;
  methods: Methods;
}

export type InferCefariWorker<T> =
  T extends CefariWorkerFactory<infer Init, infer Methods>
    ? CefariWorkerContract<Init, InferCefariWorkerMethods<Methods>>
    : never;

export type InferCefariWorkerMethods<Methods extends CefariWorkerMethods> = {
  [Name in keyof Methods]: InferCefariWorkerMethod<Methods[Name]>;
};

export type InferCefariWorkerMethod<T> =
  T extends (input: infer Input, context: CefariWorkerContext<infer Message>) => infer Output
    ? CefariWorkerMethodContract<Input, Awaited<Output>, Message>
    : never;

export type WorkerInit<T> = InferCefariWorker<T>["init"];
export type WorkerMethods<T> = InferCefariWorker<T>["methods"];

export interface WorkerStartEnvelope<Init = unknown> {
  type: "start";
  id: string;
  input: Init;
}

export interface WorkerRequestEnvelope<Input = unknown> {
  type: "request";
  requestId: string;
  method: string;
  input: Input;
}

export type WorkerStdinEnvelope<Init = unknown, Input = unknown> =
  | WorkerStartEnvelope<Init>
  | WorkerRequestEnvelope<Input>;

export type WorkerStdoutEnvelope<Output = unknown, Message = unknown> =
  | {
      type: "message";
      id: string;
      requestId: string | null;
      method: string | null;
      payload: Message;
    }
  | {
      type: "result";
      id: string;
      requestId: string;
      method: string;
      payload: Output;
    }
  | {
      type: "error";
      id: string | null;
      requestId: string | null;
      method: string | null;
      error: {
        message: string;
      };
    };

export interface WorkerProtocolIo {
  readLine(): Promise<string | null>;
  writeStdout(line: string): Promise<void> | void;
}

interface DenoLike {
  stdin: {
    readable: ReadableStream<Uint8Array>;
  };
  stdout: {
    write(data: Uint8Array): Promise<number>;
  };
}

export function defineWorker<Init, Methods extends CefariWorkerMethods>(
  definition: CefariWorkerFactory<Init, Methods>,
): CefariWorkerFactory<Init, Methods> {
  return definition;
}

export async function runCefariWorker<Init, Methods extends CefariWorkerMethods>(
  worker: CefariWorkerFactory<Init, Methods>,
  io: WorkerProtocolIo = denoProtocolIo(),
): Promise<number> {
  let workerId: string | null = null;
  try {
    const start = parseStartEnvelope<Init>(await readRequiredLine(io));
    workerId = start.id;
    const methods = await worker(start.input);
    while (true) {
      const line = await io.readLine();
      if (line === null) {
        return 0;
      }
      if (line.trim() === "") {
        continue;
      }
      await handleRequestLine(io, workerId, methods, line);
    }
  } catch (error) {
    await writeJsonLine(io, {
      type: "error",
      id: workerId,
      requestId: null,
      method: null,
      error: {
        message: error instanceof Error ? error.message : String(error),
      },
    } satisfies WorkerStdoutEnvelope);
    return 1;
  }
}

async function handleRequestLine(
  io: WorkerProtocolIo,
  workerId: string,
  methods: CefariWorkerMethods,
  line: string,
): Promise<void> {
  let request: WorkerRequestEnvelope;
  try {
    request = parseRequestEnvelope(line);
    const method = methods[request.method];
    if (method === undefined) {
      throw new Error(`worker method ${JSON.stringify(request.method)} is not defined`);
    }
    const context: CefariWorkerContext = {
      async postMessage(message) {
        await writeJsonLine(io, {
          type: "message",
          id: workerId,
          requestId: request.requestId,
          method: request.method,
          payload: message,
        } satisfies WorkerStdoutEnvelope);
      },
    };
    const result = await method(request.input, context);
    await writeJsonLine(io, {
      type: "result",
      id: workerId,
      requestId: request.requestId,
      method: request.method,
      payload: result,
    } satisfies WorkerStdoutEnvelope);
  } catch (error) {
    const parsed = requestOrNull(line);
    await writeJsonLine(io, {
      type: "error",
      id: workerId,
      requestId: parsed?.requestId ?? null,
      method: parsed?.method ?? null,
      error: {
        message: error instanceof Error ? error.message : String(error),
      },
    } satisfies WorkerStdoutEnvelope);
  }
}

async function readRequiredLine(io: WorkerProtocolIo): Promise<string> {
  const line = await io.readLine();
  if (line === null) {
    throw new Error("worker protocol input ended before start envelope");
  }
  return line;
}

function parseStartEnvelope<Init>(source: string): WorkerStartEnvelope<Init> {
  const envelope = parseEnvelope(source, "worker protocol start");
  if (envelope.type !== "start") {
    throw new Error('worker protocol start type must be "start"');
  }
  if (typeof envelope.id !== "string" || envelope.id.trim() === "") {
    throw new Error("worker protocol start id must be a non-empty string");
  }
  if (!Object.hasOwn(envelope, "input")) {
    throw new Error("worker protocol start must include input");
  }
  return {
    type: "start",
    id: envelope.id,
    input: envelope.input as Init,
  };
}

function parseRequestEnvelope(source: string): WorkerRequestEnvelope {
  const envelope = parseEnvelope(source, "worker protocol request");
  if (envelope.type !== "request") {
    throw new Error('worker protocol request type must be "request"');
  }
  if (typeof envelope.requestId !== "string" || envelope.requestId.trim() === "") {
    throw new Error("worker protocol requestId must be a non-empty string");
  }
  if (typeof envelope.method !== "string" || envelope.method.trim() === "") {
    throw new Error("worker protocol method must be a non-empty string");
  }
  if (!Object.hasOwn(envelope, "input")) {
    throw new Error("worker protocol request must include input");
  }
  return {
    type: "request",
    requestId: envelope.requestId,
    method: envelope.method,
    input: envelope.input,
  };
}

function parseEnvelope(source: string, label: string): Record<string, unknown> {
  let value: unknown;
  try {
    value = JSON.parse(source);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} must be JSON: ${message}`);
  }
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

function requestOrNull(source: string): Pick<WorkerRequestEnvelope, "requestId" | "method"> | null {
  try {
    const envelope = parseRequestEnvelope(source);
    return {
      requestId: envelope.requestId,
      method: envelope.method,
    };
  } catch {
    return null;
  }
}

async function writeJsonLine(io: WorkerProtocolIo, envelope: WorkerStdoutEnvelope): Promise<void> {
  await io.writeStdout(`${JSON.stringify(envelope)}\n`);
}

function denoProtocolIo(): WorkerProtocolIo {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  if (deno === undefined) {
    throw new Error("runCefariWorker default stdio requires Deno");
  }
  const reader = deno.stdin.readable.getReader();
  const decoder = new TextDecoder();
  let buffered = "";
  let ended = false;
  return {
    async readLine() {
      while (true) {
        const newline = buffered.indexOf("\n");
        if (newline !== -1) {
          const line = buffered.slice(0, newline).replace(/\r$/, "");
          buffered = buffered.slice(newline + 1);
          return line;
        }
        if (ended) {
          if (buffered === "") {
            return null;
          }
          const line = buffered.replace(/\r$/, "");
          buffered = "";
          return line;
        }
        const chunk = await reader.read();
        if (chunk.done) {
          ended = true;
        } else {
          buffered += decoder.decode(chunk.value, { stream: true });
        }
      }
    },
    async writeStdout(line) {
      await deno.stdout.write(new TextEncoder().encode(line));
    },
  };
}
