import type {
  CefariIpcCommand,
  CefariIpcEvent,
  CefariIpcResponse,
  CefariIpcResult,
} from "./ipc.ts";
import {
  CefariError,
  type CefariResult,
  unsupportedBridgeError,
} from "./errors.ts";

export type Unsubscribe = () => void;

export type CefariBridge = {
  invoke(command: CefariIpcCommand): Promise<CefariIpcResponse>;
  on(handler: (event: CefariIpcEvent) => void): Unsubscribe;
};

export type CefariWindow = Window & { cefari?: CefariBridge };

export function isAvailable(): boolean {
  return getBridge() !== undefined;
}

export async function invoke(
  command: CefariIpcCommand,
): Promise<CefariIpcResult> {
  const result = await tryInvoke(command);
  if (result.ok) return result.value;
  throw result.error;
}

export async function tryInvoke(
  command: CefariIpcCommand,
): Promise<CefariResult<CefariIpcResult>> {
  const bridge = getBridge();

  if (!bridge) {
    return {
      ok: false,
      error: unsupportedBridgeError(
        "window.cefari is only available inside the Cefari desktop shell",
      ),
    };
  }

  try {
    const response = await bridge.invoke(command);
    if (response.outcome.status === "ok") {
      return { ok: true, value: response.outcome.payload };
    }

    return {
      ok: false,
      error: new CefariError(response.outcome.payload, {
        command: command.command,
      }),
    };
  } catch (error) {
    return {
      ok: false,
      error: unsupportedBridgeError(
        error instanceof Error ? error.message : "native IPC transport failed",
      ),
    };
  }
}

export function onAnyEvent(
  handler: (event: CefariIpcEvent) => void,
): Unsubscribe {
  return getBridge()?.on(handler) ?? (() => {});
}

function getBridge(): CefariBridge | undefined {
  const global = globalThis as {
    cefari?: CefariBridge;
    window?: CefariWindow;
  };

  return global.window?.cefari ?? global.cefari;
}
