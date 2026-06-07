import { invokeEmpty, tryInvokeEmpty } from "./results.ts";
import type { CefariResult } from "./errors.ts";

export type AppApi = {
  quit(): Promise<void>;
  tryQuit(): Promise<CefariResult<void>>;
};

export const app: AppApi = {
  quit: (): Promise<void> => invokeEmpty({ command: "appQuit" }),
  tryQuit: (): Promise<CefariResult<void>> =>
    tryInvokeEmpty({ command: "appQuit" }),
};
