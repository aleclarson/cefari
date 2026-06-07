import type {
  CefariIpcCommand,
  CefariIpcEvent,
  CefariIpcResponse,
} from "./cefari-ipc.ts";

type CefariBridge = {
  invoke(command: CefariIpcCommand): Promise<CefariIpcResponse>;
  on(handler: (event: CefariIpcEvent) => void): () => void;
};

declare global {
  interface Window {
    cefari?: CefariBridge;
  }
}

let nextRequestId = 1;

export function invokeCefari(
  command: CefariIpcCommand,
): Promise<CefariIpcResponse> {
  if (window.cefari) {
    return window.cefari.invoke(command);
  }

  return Promise.resolve({
    id: `browser-preview-${nextRequestId++}`,
    outcome: {
      status: "err",
      payload: {
        code: "unsupported",
        details: {
          command: "bridge",
          reason: "window.cefari is only available inside the Cefari desktop shell",
        },
      },
    },
  });
}

export function onCefariEvent(handler: (event: CefariIpcEvent) => void): () => void {
  return window.cefari?.on(handler) ?? (() => {});
}
