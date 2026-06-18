export interface CefariWorkerContext<Message = unknown> {
  postMessage(message: Message): Promise<void>;
}

export interface CefariWorkerDefinition<Input = unknown, Output = unknown, Message = unknown> {
  run(input: Input, context: CefariWorkerContext<Message>): Output | Promise<Output>;
}

export interface CefariWorkerContract<Input = unknown, Output = unknown, Message = unknown> {
  input: Input;
  output: Output;
  message: Message;
}

export type InferCefariWorker<T> =
  T extends CefariWorkerDefinition<infer Input, infer Output, infer Message>
    ? CefariWorkerContract<Input, Awaited<Output>, Message>
    : never;

export type WorkerInput<T> = InferCefariWorker<T>["input"];
export type WorkerOutput<T> = InferCefariWorker<T>["output"];
export type WorkerMessage<T> = InferCefariWorker<T>["message"];

export interface WorkerStartEnvelope<Input = unknown> {
  type: "start";
  id: string;
  input: Input;
}

export type WorkerStdoutEnvelope<Output = unknown, Message = unknown> =
  | {
      type: "message";
      id: string;
      payload: Message;
    }
  | {
      type: "result";
      id: string;
      payload: Output;
    }
  | {
      type: "error";
      id: string | null;
      error: {
        message: string;
      };
    };

export interface WorkerProtocolIo {
  readStdin(): Promise<string>;
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

export function defineWorker<Input = unknown, Output = unknown, Message = unknown>(
  definition: CefariWorkerDefinition<Input, Output, Message>,
): CefariWorkerDefinition<Input, Output, Message> {
  return definition;
}

export async function runCefariWorker<Input, Output, Message>(
  worker: CefariWorkerDefinition<Input, Output, Message>,
  io: WorkerProtocolIo = denoProtocolIo(),
): Promise<number> {
  let id: string | null = null;
  try {
    const envelope = parseStartEnvelope<Input>(await io.readStdin());
    id = envelope.id;
    const context: CefariWorkerContext<Message> = {
      async postMessage(message) {
        await writeJsonLine(io, {
          type: "message",
          id: envelope.id,
          payload: message,
        } satisfies WorkerStdoutEnvelope<Output, Message>);
      },
    };
    const result = await worker.run(envelope.input, context);
    await writeJsonLine(io, {
      type: "result",
      id: envelope.id,
      payload: result,
    } satisfies WorkerStdoutEnvelope<Output, Message>);
    return 0;
  } catch (error) {
    await writeJsonLine(io, {
      type: "error",
      id,
      error: {
        message: error instanceof Error ? error.message : String(error),
      },
    } satisfies WorkerStdoutEnvelope<Output, Message>);
    return 1;
  }
}

function parseStartEnvelope<Input>(source: string): WorkerStartEnvelope<Input> {
  let value: unknown;
  try {
    value = JSON.parse(source);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`worker protocol input must be JSON: ${message}`);
  }
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("worker protocol input must be an object");
  }
  const envelope = value as Record<string, unknown>;
  if (envelope.type !== "start") {
    throw new Error('worker protocol input type must be "start"');
  }
  if (typeof envelope.id !== "string" || envelope.id.trim() === "") {
    throw new Error("worker protocol input id must be a non-empty string");
  }
  if (!Object.hasOwn(envelope, "input")) {
    throw new Error("worker protocol input must include input");
  }
  return {
    type: "start",
    id: envelope.id,
    input: envelope.input as Input,
  };
}

async function writeJsonLine(io: WorkerProtocolIo, envelope: WorkerStdoutEnvelope): Promise<void> {
  await io.writeStdout(`${JSON.stringify(envelope)}\n`);
}

function denoProtocolIo(): WorkerProtocolIo {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  if (deno === undefined) {
    throw new Error("runCefariWorker default stdio requires Deno");
  }
  return {
    async readStdin() {
      return await new Response(deno.stdin.readable).text();
    },
    async writeStdout(line) {
      await deno.stdout.write(new TextEncoder().encode(line));
    },
  };
}
