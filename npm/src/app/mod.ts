export type * from "./ipc.ts";
export type { AppApi } from "./app.ts";
export { CefariError, type CefariResult, isCefariError } from "./errors.ts";
export type { CefariBridge, CefariWindow, Unsubscribe } from "./transport.ts";
export { invoke, isAvailable, onAnyEvent, tryInvoke } from "./transport.ts";
export { type CefariEventMap, type CefariEventName, on } from "./events.ts";
export {
  dialogs,
  type DialogOptions,
  type DialogOutcome,
  type DialogsApi,
} from "./dialogs.ts";
export { downloads, type DownloadsApi } from "./downloads.ts";
export { files, type FilesApi, type JsonValue } from "./files.ts";
export {
  CefariDirent,
  CefariStats,
  type FileData,
  type FileEncoding,
  type FileSystemApi,
  fs,
} from "./fs.ts";
export { app } from "./app.ts";
export type {
  WindowApi,
  WindowCreateOptions,
  WindowTargetInput,
} from "./window.ts";
export { windowControls as window } from "./window.ts";
export type { WindowEventFilter, WindowsApi } from "./windows.ts";
export { windows } from "./windows.ts";
export type { ShellApi } from "./shell.ts";
export { shell } from "./shell.ts";
export type { UpdateApplyOptions, UpdatesApi } from "./updates.ts";
export { updates } from "./updates.ts";
export type { ServiceApi } from "./service.ts";
export { service } from "./service.ts";
export type { TrayApi } from "./tray.ts";
export { tray } from "./tray.ts";
export {
  type NotificationPermission,
  notifications,
  type NotificationsApi,
  type NotificationSent,
  type SendNotificationInput,
} from "./notifications.ts";

import { app, type AppApi } from "./app.ts";
import { dialogs, type DialogsApi } from "./dialogs.ts";
import { downloads, type DownloadsApi } from "./downloads.ts";
import { type CefariEventMap, on } from "./events.ts";
import type { CefariResult } from "./errors.ts";
import { files, type FilesApi } from "./files.ts";
import { type FileSystemApi, fs } from "./fs.ts";
import type {
  CefariIpcCommand,
  CefariIpcEvent,
  CefariIpcResult,
} from "./ipc.ts";
import { notifications, type NotificationsApi } from "./notifications.ts";
import { service, type ServiceApi } from "./service.ts";
import { shell, type ShellApi } from "./shell.ts";
import { tray, type TrayApi } from "./tray.ts";
import { invoke, isAvailable, onAnyEvent, tryInvoke } from "./transport.ts";
import type { Unsubscribe } from "./transport.ts";
import { updates, type UpdatesApi } from "./updates.ts";
import { type WindowApi, windowControls } from "./window.ts";
import { windows, type WindowsApi } from "./windows.ts";

export type CefariApp = {
  isAvailable(): boolean;
  invoke(command: CefariIpcCommand): Promise<CefariIpcResult>;
  tryInvoke(command: CefariIpcCommand): Promise<CefariResult<CefariIpcResult>>;
  on<Name extends keyof CefariEventMap>(
    name: Name,
    handler: (payload: CefariEventMap[Name]) => void,
  ): Unsubscribe;
  onAnyEvent(handler: (event: CefariIpcEvent) => void): Unsubscribe;
  app: AppApi;
  window: WindowApi;
  windows: WindowsApi;
  shell: ShellApi;
  updates: UpdatesApi;
  service: ServiceApi;
  tray: TrayApi;
  notifications: NotificationsApi;
  dialogs: DialogsApi;
  downloads: DownloadsApi;
  fs: FileSystemApi;
  files: FilesApi;
};

export const cefari: Readonly<CefariApp> = Object.freeze({
  isAvailable,
  invoke,
  tryInvoke,
  on,
  onAnyEvent,
  app,
  window: windowControls,
  windows,
  shell,
  updates,
  service,
  tray,
  notifications,
  dialogs,
  downloads,
  fs,
  files,
});
