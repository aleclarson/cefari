import { on } from "./events.ts";
import { invokeUpdateCheck, invokeUpdateState } from "./results.ts";
import type { UpdateCheckResult, UpdateStateResult } from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type UpdatesApi = {
  state(): Promise<UpdateStateResult>;
  check(): Promise<UpdateCheckResult>;
  onStateChanged(handler: (state: UpdateStateResult) => void): Unsubscribe;
};

export const updates: UpdatesApi = {
  state: (): Promise<UpdateStateResult> =>
    invokeUpdateState({ command: "updateState" }),
  check: (): Promise<UpdateCheckResult> =>
    invokeUpdateCheck({ command: "updateCheck" }),
  onStateChanged: (handler: (state: UpdateStateResult) => void): Unsubscribe =>
    on("updateStateChanged", handler),
};
