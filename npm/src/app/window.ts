import { on } from "./events.ts";
import type { WindowIdEvent, WindowState } from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";
import {
  type WindowCreateOptions,
  windows,
  type WindowTargetInput,
} from "./windows.ts";

export type { WindowCreateOptions, WindowTargetInput } from "./windows.ts";

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
  current: (): Promise<WindowState> => windows.current(),
  list: async (): Promise<WindowState[]> => windows.list(),
  create: (options: WindowCreateOptions = {}): Promise<WindowState> =>
    windows.create(options),
  show: (target?: WindowTargetInput): Promise<WindowState> =>
    windows.show(target ?? null),
  focus: (target?: WindowTargetInput): Promise<WindowState> =>
    windows.focus(target ?? null),
  close: (target?: WindowTargetInput): Promise<WindowState> =>
    windows.close(target ?? null),
  setTitle: (title: string, target?: WindowTargetInput): Promise<WindowState> =>
    windows.setTitle(target ?? null, title),
  onShown: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowShown", (event) => handler(event.state)),
  onFocused: (handler: (state: WindowState) => void): Unsubscribe =>
    on("windowFocused", (event) => handler(event.state)),
  onClosed: (handler: (event: WindowIdEvent) => void): Unsubscribe =>
    on("windowClosed", handler),
};
