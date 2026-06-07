import {
  invokeEmpty,
  invokeExternalUrl,
  invokeReloadUi,
  tryInvokeEmpty,
} from "./results.ts";
import type { CefariResult } from "./errors.ts";
import type { ExternalUrlResult } from "./ipc.ts";

export type ShellApi = {
  openLogs(): Promise<void>;
  tryOpenLogs(): Promise<CefariResult<void>>;
  reloadUi(): Promise<void>;
  openExternalUrl(url: string | URL): Promise<ExternalUrlResult>;
};

export const shell: ShellApi = {
  openLogs: (): Promise<void> => invokeEmpty({ command: "openLogs" }),
  tryOpenLogs: (): Promise<CefariResult<void>> =>
    tryInvokeEmpty({ command: "openLogs" }),
  reloadUi: (): Promise<void> => invokeReloadUi({ command: "reloadUi" }),
  openExternalUrl: (url: string | URL): Promise<ExternalUrlResult> =>
    invokeExternalUrl({
      command: "openExternalUrl",
      payload: { url: url.toString() },
    }),
};
