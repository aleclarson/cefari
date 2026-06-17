import type {
  CefariIpcEvent,
  DeepLinkOpenEvent,
  DownloadCanceledEvent,
  DownloadCompletedEvent,
  DownloadFailedEvent,
  DownloadProgressEvent,
  DownloadStartedEvent,
  NotificationResponseEvent,
  ServiceStatusResult,
  UpdateStateResult,
  WindowState,
} from "./ipc.ts";
import { onAnyEvent, type Unsubscribe } from "./transport.ts";

export type CefariEventMap = {
  windowShown: WindowState;
  windowFocused: WindowState;
  windowClosed: undefined;
  deepLinkOpened: DeepLinkOpenEvent;
  trayRestoreWindow: undefined;
  updateStateChanged: UpdateStateResult;
  serviceStatusChanged: ServiceStatusResult;
  "download.started": DownloadStartedEvent;
  "download.progress": DownloadProgressEvent;
  "download.completed": DownloadCompletedEvent;
  "download.canceled": DownloadCanceledEvent;
  "download.failed": DownloadFailedEvent;
  "notification.response": NotificationResponseEvent;
};

export type CefariEventName = keyof CefariEventMap;

export function on<Name extends CefariEventName>(
  name: Name,
  handler: (payload: CefariEventMap[Name]) => void,
): Unsubscribe {
  return onAnyEvent((event) => {
    const payload = eventPayload(name, event);
    if (payload.matched) {
      handler(payload.value as CefariEventMap[Name]);
    }
  });
}

export { onAnyEvent };

function eventPayload(
  name: CefariEventName,
  event: CefariIpcEvent,
): { matched: true; value: unknown } | { matched: false } {
  switch (name) {
    case "windowShown":
      return event.event === "windowShown"
        ? { matched: true, value: event.payload }
        : { matched: false };
    case "windowFocused":
      return event.event === "windowFocused"
        ? { matched: true, value: event.payload }
        : { matched: false };
    case "windowClosed":
      return event.event === "windowClosed"
        ? { matched: true, value: undefined }
        : { matched: false };
    case "deepLinkOpened":
      return event.event === "deepLinkOpened"
        ? { matched: true, value: event.payload }
        : { matched: false };
    case "trayRestoreWindow":
      return event.event === "trayRestoreWindow"
        ? { matched: true, value: undefined }
        : { matched: false };
    case "updateStateChanged":
      return event.event === "updateStateChanged"
        ? { matched: true, value: event.payload }
        : { matched: false };
    case "serviceStatusChanged":
      return event.event === "serviceStatusChanged"
        ? { matched: true, value: event.payload }
        : { matched: false };
    case "download.started":
      return event.event === "download" && event.payload.event === "started"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
    case "download.progress":
      return event.event === "download" && event.payload.event === "progress"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
    case "download.completed":
      return event.event === "download" && event.payload.event === "completed"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
    case "download.canceled":
      return event.event === "download" && event.payload.event === "canceled"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
    case "download.failed":
      return event.event === "download" && event.payload.event === "failed"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
    case "notification.response":
      return event.event === "notification" &&
          event.payload.event === "response"
        ? { matched: true, value: event.payload.payload }
        : { matched: false };
  }
}
