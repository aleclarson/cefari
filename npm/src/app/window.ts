import { on } from "./events.ts";
import { invokeWindow, invokeWindowList } from "./results.ts";
import type {
  WindowCreateRequest,
  WindowIdEvent,
  WindowState,
  WindowTarget,
  WindowTargetRequest,
} from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type WindowTargetInput = string | WindowTarget | null;
export type WindowCreateOptions = Partial<WindowCreateRequest>;

export type WindowApi = {
  current(): Promise<WindowState>;
  list(): Promise<WindowState[]>;
  create(options?: WindowCreateOptions): Promise<WindowState>;
  show(target?: WindowTargetInput): Promise<WindowState>;
  focus(target?: WindowTargetInput): Promise<WindowState>;
  close(target?: WindowTargetInput): Promise<WindowState>;
  setTitle(title: string, target?: WindowTargetInput): Promise<WindowState>;
  onShown(handler: (state: WindowState) => void): Unsubscribe;
  onFocused(handler: (state: WindowState) => void): Unsubscribe;
  onClosed(handler: (event: WindowIdEvent) => void): Unsubscribe;
};

export const windowControls: WindowApi = {
  current: (): Promise<WindowState> =>
    invokeWindow({ command: "windowCurrent" }),
  list: async (): Promise<WindowState[]> =>
    (await invokeWindowList({ command: "windowList" })).windows,
  create: (options: WindowCreateOptions = {}): Promise<WindowState> =>
    invokeWindow({ command: "windowCreate", payload: createRequest(options) }),
  show: (target?: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowShow", payload: targetRequest(target) }),
  focus: (target?: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowFocus", payload: targetRequest(target) }),
  close: (target?: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowClose", payload: targetRequest(target) }),
  setTitle: (title: string, target?: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({
      command: "windowSetTitle",
      payload: { ...targetRequest(target), title },
    }),
  onShown: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowShown", (event) => handler(event.state)),
  onFocused: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowFocused", (event) => handler(event.state)),
  onClosed: (handler: (event: WindowIdEvent) => void): Unsubscribe =>
    on("windowClosed", handler),
};

function targetRequest(target?: WindowTargetInput): WindowTargetRequest {
  if (target == null) return { target: null };
  if (typeof target === "string") return { target: { id: target } };
  return { target: { id: target.id ?? null } };
}

function createRequest(options: WindowCreateOptions): WindowCreateRequest {
  return {
    id: options.id ?? null,
    route: options.route ?? null,
    title: options.title ?? null,
    width: options.width ?? null,
    height: options.height ?? null,
    minWidth: options.minWidth ?? null,
    minHeight: options.minHeight ?? null,
    maxWidth: options.maxWidth ?? null,
    maxHeight: options.maxHeight ?? null,
    x: options.x ?? null,
    y: options.y ?? null,
    visible: options.visible ?? null,
    focused: options.focused ?? null,
    resizable: options.resizable ?? null,
    decorations: options.decorations ?? null,
    alwaysOnTop: options.alwaysOnTop ?? null,
    parentId: options.parentId ?? null,
    modal: options.modal ?? null,
    persistKey: options.persistKey ?? null,
  };
}
