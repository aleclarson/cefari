import { on } from "./events.ts";
import { invokeWindow } from "./results.ts";
import type { WindowState } from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type WindowApi = {
  show(): Promise<WindowState>;
  focus(): Promise<WindowState>;
  close(): Promise<WindowState>;
  setTitle(title: string): Promise<WindowState>;
  onShown(handler: (state: WindowState) => void): Unsubscribe;
  onFocused(handler: (state: WindowState) => void): Unsubscribe;
  onClosed(handler: () => void): Unsubscribe;
};

export const windowControls: WindowApi = {
  show: (): Promise<WindowState> => invokeWindow({ command: "windowShow" }),
  focus: (): Promise<WindowState> => invokeWindow({ command: "windowFocus" }),
  close: (): Promise<WindowState> => invokeWindow({ command: "windowClose" }),
  setTitle: (title: string): Promise<WindowState> =>
    invokeWindow({ command: "windowSetTitle", payload: { title } }),
  onShown: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowShown", handler),
  onFocused: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowFocused", handler),
  onClosed: (handler: () => void): Unsubscribe =>
    on("windowClosed", () => handler()),
};
