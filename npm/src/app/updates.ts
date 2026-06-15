import { on } from "./events.ts";
import {
  invokeEmpty,
  invokeUpdateApply,
  invokeUpdateCheck,
  invokeUpdateState,
} from "./results.ts";
import type {
  UpdateApplyResult,
  UpdateCheckResult,
  UpdateStateResult,
} from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type UpdateApplyOptions = {
  updateId?: string | null;
};

export type UpdatesApi = {
  state(): Promise<UpdateStateResult>;
  check(): Promise<UpdateCheckResult>;
  apply(options?: UpdateApplyOptions): Promise<UpdateApplyResult>;
  restart(): Promise<void>;
  applyAndRestart(options?: UpdateApplyOptions): Promise<void>;
  onStateChanged(handler: (state: UpdateStateResult) => void): Unsubscribe;
};

export const updates: UpdatesApi = {
  state: (): Promise<UpdateStateResult> =>
    invokeUpdateState({ command: "updateState" }),
  check: (): Promise<UpdateCheckResult> =>
    invokeUpdateCheck({ command: "updateCheck" }),
  apply: (options: UpdateApplyOptions = {}): Promise<UpdateApplyResult> =>
    invokeUpdateApply({
      command: "updateApply",
      payload: { updateId: options.updateId ?? null },
    }),
  restart: (): Promise<void> => invokeEmpty({ command: "updateRestart" }),
  applyAndRestart: async (options: UpdateApplyOptions = {}): Promise<void> => {
    const result = await updates.apply(options);
    if (result.restartRequired) {
      await updates.restart();
    }
  },
  onStateChanged: (handler: (state: UpdateStateResult) => void): Unsubscribe =>
    on("updateStateChanged", handler),
};
