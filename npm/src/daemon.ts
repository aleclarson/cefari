export type DaemonConnection = {
  readable: ReadableStream<Uint8Array>;
  writable: WritableStream<Uint8Array>;
  close(): Promise<void>;
  closed: Promise<void>;
};

export interface CefariDaemonResources {
  resourceDir: string;
  nativeDir: string;
  native: Record<string, string>;
}

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

const CEFARI_DAEMON_RESOURCES_ENV = "CEFARI_DAEMON_RESOURCES";

export function isCefariDaemon(): boolean {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  try {
    return deno?.env?.get("CEFARI_DAEMON") === "1";
  } catch {
    return false;
  }
}

export function getDaemonResources(): CefariDaemonResources {
  const deno = (globalThis as { Deno?: DenoLike }).Deno;
  let value: string | undefined;
  try {
    value = deno?.env?.get(CEFARI_DAEMON_RESOURCES_ENV);
  } catch {
    value = undefined;
  }
  if (value === undefined || value === "") {
    throw new Error("Cefari daemon resources are unavailable outside a configured Cefari daemon");
  }
  return parseDaemonResources(value);
}

export function daemonNativePath(id: string): string {
  const path = getDaemonResources().native[id];
  if (path === undefined) {
    throw new Error(`daemon native resource ${JSON.stringify(id)} is not configured`);
  }
  return path;
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

function parseDaemonResources(source: string): CefariDaemonResources {
  const parsed = JSON.parse(source) as {
    resourceDir?: unknown;
    nativeDir?: unknown;
    native?: unknown;
  };
  if (typeof parsed.resourceDir !== "string" || parsed.resourceDir.trim() === "") {
    throw new Error("CEFARI_DAEMON_RESOURCES.resourceDir must be a non-empty string");
  }
  if (typeof parsed.nativeDir !== "string" || parsed.nativeDir.trim() === "") {
    throw new Error("CEFARI_DAEMON_RESOURCES.nativeDir must be a non-empty string");
  }
  if (parsed.native === null || typeof parsed.native !== "object" || Array.isArray(parsed.native)) {
    throw new Error("CEFARI_DAEMON_RESOURCES.native must be an object");
  }
  return {
    resourceDir: parsed.resourceDir,
    nativeDir: parsed.nativeDir,
    native: Object.fromEntries(
      Object.entries(parsed.native).map(([id, path]) => {
        if (typeof path !== "string" || path.trim() === "") {
          throw new Error(`CEFARI_DAEMON_RESOURCES.native.${id} must be a non-empty string`);
        }
        return [id, path];
      }),
    ),
  };
}
