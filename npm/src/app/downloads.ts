import { invokeDownload } from "./results.ts";
import type { DownloadResult } from "./ipc.ts";

export type DownloadsApi = {
  cancel(id: string): Promise<DownloadResult>;
  reveal(id: string): Promise<DownloadResult>;
};

export const downloads: DownloadsApi = {
  cancel: (id: string): Promise<DownloadResult> =>
    invokeDownload({
      command: "download",
      payload: { download: "cancel", payload: { id } },
    }),
  reveal: (id: string): Promise<DownloadResult> =>
    invokeDownload({
      command: "download",
      payload: { download: "reveal", payload: { id } },
    }),
};
