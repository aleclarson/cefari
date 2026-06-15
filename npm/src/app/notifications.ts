import { on } from "./events.ts";
import { CefariError } from "./errors.ts";
import { invokeNotification } from "./results.ts";
import type {
  NotificationResponseEvent,
  NotificationResult,
  NotificationSendRequest,
} from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type NotificationPermission = { allowed: boolean };
export type NotificationSent = { id: string };

export type SendNotificationInput = {
  title: string;
  body?: string | null;
};

export type NotificationsApi = {
  permissionState(): Promise<NotificationPermission>;
  requestPermission(): Promise<NotificationPermission>;
  send(input: SendNotificationInput): Promise<NotificationSent>;
  onResponse(handler: (event: NotificationResponseEvent) => void): Unsubscribe;
};

export const notifications: NotificationsApi = {
  permissionState: async (): Promise<NotificationPermission> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "permissionState" },
    });
    return permissionFromResult(result);
  },
  requestPermission: async (): Promise<NotificationPermission> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "requestPermission" },
    });
    return permissionFromResult(result);
  },
  send: async (input: SendNotificationInput): Promise<NotificationSent> => {
    const payload: NotificationSendRequest = {
      title: input.title,
      body: input.body ?? null,
    };
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "send", payload },
    });
    if (result.result === "sent") return result.payload;
    return unsupportedNotificationResult(result);
  },
  onResponse: (
    handler: (event: NotificationResponseEvent) => void,
  ): Unsubscribe => on("notification.response", handler),
};

function permissionFromResult(
  result: NotificationResult,
): NotificationPermission {
  switch (result.result) {
    case "permissionState":
    case "permissionRequested":
      return result.payload;
    case "permissionDenied":
      return { allowed: false };
    case "unsupported":
      throw new CefariError({
        code: "unsupported",
        details: {
          command: "notification",
          reason: result.payload.reason,
        },
      });
    case "sent":
      throw new CefariError({
        code: "invalidCommand",
        details: {
          message: "unexpected notification sent result",
        },
      });
  }
}

function unsupportedNotificationResult(
  result: NotificationResult,
): never {
  if (result.result === "unsupported") {
    throw new CefariError({
      code: "unsupported",
      details: {
        command: "notification.send",
        reason: result.payload.reason,
      },
    });
  }
  if (result.result === "permissionDenied") {
    throw new CefariError({
      code: "denied",
      details: {
        message: "notification permission denied",
      },
    });
  }
  throw new CefariError({
    code: "invalidCommand",
    details: {
      message: `unexpected notification result: ${result.result}`,
    },
  });
}
