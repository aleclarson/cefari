export type DaemonConnection = {
  readable: ReadableStream<Uint8Array>;
  writable: WritableStream<Uint8Array>;
  close(): Promise<void>;
  closed: Promise<void>;
};

type DenoLike = {
  env?: {
    get(name: string): string | undefined;
  };
  stdin?: {
    readable: ReadableStream<Uint8Array>;
  };
  stdout?: {
    writable: WritableStream<Uint8Array>;
  };
};

export function isCefariDaemon(): boolean {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  try {
    return deno?.env?.get("CEFARI_DAEMON") === "1";
  } catch {
    return false;
  }
}

export function connect(): DaemonConnection {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  if (
    deno?.stdin?.readable === undefined || deno.stdout?.writable === undefined
  ) {
    throw new Error("cefari/daemon requires Deno stdin and stdout streams");
  }

  let resolveClosed: () => void = () => {};
  const closed = new Promise<void>((resolve) => {
    resolveClosed = resolve;
  });

  return {
    readable: deno.stdin.readable,
    writable: deno.stdout.writable,
    async close() {
      resolveClosed();
    },
    closed,
  };
}
