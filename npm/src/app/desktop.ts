import { daemon, type DaemonApi } from "./daemon.ts";
import { service, type ServiceApi } from "./service.ts";
import { tray, type TrayApi } from "./tray.ts";
import { updates, type UpdatesApi } from "./updates.ts";
import { type WindowApi, windowControls } from "./window.ts";
import { windows, type WindowsApi } from "./windows.ts";

export type DesktopApi = {
  window: WindowApi;
  windows: WindowsApi;
  updates: UpdatesApi;
  service: ServiceApi;
  tray: TrayApi;
  daemon: DaemonApi;
};

export const desktop: Readonly<DesktopApi> = Object.freeze({
  window: windowControls,
  windows,
  updates,
  service,
  tray,
  daemon,
});
