import { on } from "./events.ts";
import { invokeTray } from "./results.ts";
import type { TrayResult } from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type TrayApi = {
  restoreWindow(): Promise<TrayResult>;
  onRestoreWindow(handler: () => void): Unsubscribe;
};

export const tray: TrayApi = {
  restoreWindow: (): Promise<TrayResult> =>
    invokeTray({ command: "trayRestoreWindow" }),
  onRestoreWindow: (handler: () => void): Unsubscribe =>
    on("trayRestoreWindow", () => handler()),
};
