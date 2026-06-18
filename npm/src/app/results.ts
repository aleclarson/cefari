import { unexpectedResultError } from "./errors.ts";
import type {
  CefariIpcCommand,
  CefariIpcResult,
  DialogResult,
  DownloadResult,
  ExternalUrlResult,
  FileResult,
  NotificationResult,
  ServiceStatusResult,
  TrayResult,
  UpdateApplyResult,
  UpdateCheckResult,
  UpdateStateResult,
  WorkerResult,
  WindowListResult,
  WindowState,
} from "./ipc.ts";
import { invoke, tryInvoke } from "./transport.ts";
import type { CefariResult } from "./errors.ts";

type ResultByTag<Tag extends CefariIpcResult["result"]> = Extract<
  CefariIpcResult,
  { result: Tag }
>;

export async function invokeResult<Tag extends CefariIpcResult["result"]>(
  command: CefariIpcCommand,
  tag: Tag,
): Promise<ResultByTag<Tag>> {
  const result = await invoke(command);
  if (result.result === tag) return result as ResultByTag<Tag>;
  throw unexpectedResultError(command, tag, result);
}

export async function tryInvokeResult<Tag extends CefariIpcResult["result"]>(
  command: CefariIpcCommand,
  tag: Tag,
): Promise<CefariResult<ResultByTag<Tag>>> {
  const result = await tryInvoke(command);
  if (!result.ok) return result;
  if (result.value.result === tag) {
    return { ok: true, value: result.value as ResultByTag<Tag> };
  }
  return {
    ok: false,
    error: unexpectedResultError(command, tag, result.value),
  };
}

export async function invokeEmpty(command: CefariIpcCommand): Promise<void> {
  await invokeResult(command, "empty");
}

export async function tryInvokeEmpty(
  command: CefariIpcCommand,
): Promise<CefariResult<void>> {
  const result = await tryInvokeResult(command, "empty");
  if (!result.ok) return result;
  return { ok: true, value: undefined };
}

export async function invokeWindow(
  command: CefariIpcCommand,
): Promise<WindowState> {
  return (await invokeResult(command, "window")).payload;
}

export async function invokeWindowList(
  command: CefariIpcCommand,
): Promise<WindowListResult> {
  return (await invokeResult(command, "windowList")).payload;
}

export async function invokeExternalUrl(
  command: CefariIpcCommand,
): Promise<ExternalUrlResult> {
  return (await invokeResult(command, "externalUrl")).payload;
}

export async function invokeUpdateState(
  command: CefariIpcCommand,
): Promise<UpdateStateResult> {
  return (await invokeResult(command, "updateState")).payload;
}

export async function invokeUpdateCheck(
  command: CefariIpcCommand,
): Promise<UpdateCheckResult> {
  return (await invokeResult(command, "updateCheck")).payload;
}

export async function invokeUpdateApply(
  command: CefariIpcCommand,
): Promise<UpdateApplyResult> {
  return (await invokeResult(command, "updateApply")).payload;
}

export async function invokeServiceStatus(
  command: CefariIpcCommand,
): Promise<ServiceStatusResult> {
  return (await invokeResult(command, "serviceStatus")).payload;
}

export async function invokeTray(
  command: CefariIpcCommand,
): Promise<TrayResult> {
  return (await invokeResult(command, "tray")).payload;
}

export async function invokeNotification(
  command: CefariIpcCommand,
): Promise<NotificationResult> {
  return (await invokeResult(command, "notification")).payload;
}

export async function invokeDialog(
  command: CefariIpcCommand,
): Promise<DialogResult> {
  return (await invokeResult(command, "dialog")).payload;
}

export async function invokeDownload(
  command: CefariIpcCommand,
): Promise<DownloadResult> {
  return (await invokeResult(command, "download")).payload;
}

export async function invokeFile(
  command: CefariIpcCommand,
): Promise<FileResult> {
  return (await invokeResult(command, "file")).payload;
}

export async function invokeWorker(
  command: CefariIpcCommand,
): Promise<WorkerResult> {
  return (await invokeResult(command, "worker")).payload;
}
