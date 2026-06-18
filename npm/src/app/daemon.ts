import { CefariError, unsupportedBridgeError } from "./errors.ts";
import type { CefariIpcError } from "./ipc.ts";

export type DaemonConnection = {
  readable: ReadableStream<Uint8Array>;
  writable: WritableStream<Uint8Array>;
  close(): Promise<void>;
  closed: Promise<void>;
};

export type DaemonApi = {
  isConfigured(): boolean;
  connect(): Promise<DaemonConnection>;
};

type DaemonBridgeCommand =
  | { op: "connect" }
  | { op: "write"; connectionId: number; chunkBase64: string }
  | { op: "closeWrite"; connectionId: number }
  | { op: "close"; connectionId: number };

type DaemonBridgeEvent =
  | { event: "chunk"; connectionId: number; chunkBase64: string }
  | { event: "close"; connectionId: number }
  | { event: "error"; connectionId: number; message: string };

type DaemonBridgeResponse = {
  outcome:
    | { status: "ok"; payload: { connectionId?: number } }
    | { status: "err"; payload: CefariIpcError };
};

type DaemonBridgeHooks = {
  post(command: DaemonBridgeCommand): Promise<DaemonBridgeResponse>;
  on(handler: (event: DaemonBridgeEvent) => void): () => void;
};

export const daemon: DaemonApi = {
  isConfigured(): boolean {
    return daemonBridgeHooks() !== undefined;
  },

  async connect(): Promise<DaemonConnection> {
    const hooks = daemonBridgeHooks();
    if (hooks === undefined) {
      throw unsupportedBridgeError(
        "window.cefari daemon streams are only available inside the Cefari desktop shell",
      );
    }

    const response = await hooks.post({ op: "connect" });
    const payload = unwrapDaemonResponse(response);
    const connectionId = payload.connectionId;
    if (typeof connectionId !== "number") {
      throw unsupportedBridgeError(
        "daemon stream connect response did not include a connection id",
      );
    }

    let unsubscribe = () => {};
    let closeReadable: (() => void) | undefined;
    let errorReadable: ((error: unknown) => void) | undefined;
    let resolveClosed: () => void = () => {};
    let rejectClosed: (error: unknown) => void = () => {};
    let closed = false;
    const closedPromise = new Promise<void>((resolve, reject) => {
      resolveClosed = resolve;
      rejectClosed = reject;
    });

    const readable = new ReadableStream<Uint8Array>({
      start(controller) {
        closeReadable = () => controller.close();
        errorReadable = (error) => controller.error(error);
        unsubscribe = hooks.on((event) => {
          if (event.connectionId !== connectionId) return;
          if (event.event === "chunk") {
            controller.enqueue(decodeBase64(event.chunkBase64));
          } else if (event.event === "close") {
            closed = true;
            unsubscribe();
            controller.close();
            resolveClosed();
          } else {
            const error = unsupportedBridgeError(event.message);
            closed = true;
            unsubscribe();
            controller.error(error);
            rejectClosed(error);
          }
        });
      },
      async cancel() {
        unsubscribe();
        if (closed) return;
        try {
          unwrapDaemonResponse(await hooks.post({ op: "close", connectionId }));
          closed = true;
          resolveClosed();
        } catch (error) {
          closed = true;
          rejectClosed(error);
          throw error;
        }
      },
    });

    const writable = new WritableStream<Uint8Array>({
      async write(chunk) {
        unwrapDaemonResponse(
          await hooks.post({
            op: "write",
            connectionId,
            chunkBase64: encodeBase64(chunk),
          }),
        );
      },
      async close() {
        unwrapDaemonResponse(
          await hooks.post({ op: "closeWrite", connectionId }),
        );
      },
      async abort(reason) {
        unsubscribe();
        if (!closed) {
          closed = true;
          rejectClosed(reason);
        }
        unwrapDaemonResponse(await hooks.post({ op: "close", connectionId }));
      },
    });

    return {
      readable,
      writable,
      closed: closedPromise,
      async close() {
        unsubscribe();
        if (closed) return;
        try {
          unwrapDaemonResponse(await hooks.post({ op: "close", connectionId }));
          closed = true;
          closeReadable?.();
          resolveClosed();
        } catch (error) {
          closed = true;
          rejectClosed(error);
          errorReadable?.(error);
          throw error;
        }
      },
    };
  },
};

function daemonBridgeHooks(): DaemonBridgeHooks | undefined {
  const global = globalThis as {
    window?: unknown;
    __CEFARI_DAEMON_STREAM_POST__?: DaemonBridgeHooks["post"];
    __CEFARI_DAEMON_STREAM_ON__?: DaemonBridgeHooks["on"];
  };
  const window = global.window as
    | {
      __CEFARI_DAEMON_STREAM_POST__?: DaemonBridgeHooks["post"];
      __CEFARI_DAEMON_STREAM_ON__?: DaemonBridgeHooks["on"];
    }
    | undefined;
  const post = window?.__CEFARI_DAEMON_STREAM_POST__ ??
    global.__CEFARI_DAEMON_STREAM_POST__;
  const on = window?.__CEFARI_DAEMON_STREAM_ON__ ??
    global.__CEFARI_DAEMON_STREAM_ON__;
  return typeof post === "function" && typeof on === "function"
    ? { post, on }
    : undefined;
}

function unwrapDaemonResponse(
  response: DaemonBridgeResponse,
): { connectionId?: number } {
  if (response.outcome.status === "ok") {
    return response.outcome.payload;
  }
  throw new CefariError(response.outcome.payload, { command: "daemon" });
}

function encodeBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return btoa(binary);
}

function decodeBase64(value: string): Uint8Array {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}
