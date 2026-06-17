import { on } from "./events.ts";
import { invokeWindow, invokeWindowList } from "./results.ts";
import type {
  WindowCreateRequest,
  WindowIdEvent,
  WindowState,
  WindowStateEvent,
  WindowTarget,
  WindowTargetRequest,
} from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type WindowTargetInput = string | WindowTarget | WindowState | null;
export type WindowCreateOptions = Partial<WindowCreateRequest>;
export type WindowEventFilter = {
  windowId?: string | null;
};

export type WindowsApi = {
  current(): Promise<WindowState>;
  list(): Promise<WindowState[]>;
  get(target: WindowTargetInput): Promise<WindowState | undefined>;
  create(options?: WindowCreateOptions): Promise<WindowState>;
  show(target: WindowTargetInput): Promise<WindowState>;
  focus(target: WindowTargetInput): Promise<WindowState>;
  close(target: WindowTargetInput): Promise<WindowState>;
  setTitle(target: WindowTargetInput, title: string): Promise<WindowState>;
  onCreated(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onShown(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onFocused(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onBlurred(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onCloseRequested(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onClosed(
    handler: (event: WindowIdEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onMoved(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onResized(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
  onTitleChanged(
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe;
};

export const windows: WindowsApi = {
  current: (): Promise<WindowState> =>
    invokeWindow({ command: "windowCurrent" }),
  list: async (): Promise<WindowState[]> =>
    (await invokeWindowList({ command: "windowList" })).windows,
  get: async (target: WindowTargetInput): Promise<WindowState | undefined> => {
    const id = targetId(target);
    if (id == null) return windows.current();
    return (await windows.list()).find((window) => window.id === id);
  },
  create: (options: WindowCreateOptions = {}): Promise<WindowState> =>
    invokeWindow({ command: "windowCreate", payload: createRequest(options) }),
  show: (target: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowShow", payload: targetRequest(target) }),
  focus: (target: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowFocus", payload: targetRequest(target) }),
  close: (target: WindowTargetInput): Promise<WindowState> =>
    invokeWindow({ command: "windowClose", payload: targetRequest(target) }),
  setTitle: (
    target: WindowTargetInput,
    title: string,
  ): Promise<WindowState> =>
    invokeWindow({
      command: "windowSetTitle",
      payload: { ...targetRequest(target), title },
    }),
  onCreated: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowCreated", handler, filter),
  onShown: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowShown", handler, filter),
  onFocused: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowFocused", handler, filter),
  onBlurred: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowBlurred", handler, filter),
  onCloseRequested: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowCloseRequested", handler, filter),
  onClosed: (
    handler: (event: WindowIdEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe =>
    on("windowClosed", (event) => {
      if (matchesFilter(filter, event.windowId)) handler(event);
    }),
  onMoved: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowMoved", handler, filter),
  onResized: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowResized", handler, filter),
  onTitleChanged: (
    handler: (event: WindowStateEvent) => void,
    filter?: WindowEventFilter,
  ): Unsubscribe => onWindowStateEvent("windowTitleChanged", handler, filter),
};

function targetRequest(target?: WindowTargetInput): WindowTargetRequest {
  const id = targetId(target);
  return { target: id == null ? null : { id } };
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

function targetId(target?: WindowTargetInput): string | null {
  if (target == null) return null;
  if (typeof target === "string") return target;
  return target.id ?? null;
}

function onWindowStateEvent(
  name:
    | "windowCreated"
    | "windowShown"
    | "windowFocused"
    | "windowBlurred"
    | "windowCloseRequested"
    | "windowMoved"
    | "windowResized"
    | "windowTitleChanged",
  handler: (event: WindowStateEvent) => void,
  filter?: WindowEventFilter,
): Unsubscribe {
  return on(name, (event) => {
    if (matchesFilter(filter, event.windowId)) handler(event);
  });
}

function matchesFilter(
  filter: WindowEventFilter | undefined,
  windowId: string,
): boolean {
  return filter?.windowId == null || filter.windowId === windowId;
}
