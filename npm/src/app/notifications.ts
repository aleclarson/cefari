import { on } from "./events.ts";
import { CefariError } from "./errors.ts";
import { invokeNotification } from "./results.ts";
import type {
  ActiveNotification,
  NotificationCapabilities,
  NotificationCategory,
  NotificationMediaReference,
  NotificationResponseEvent,
  NotificationResult,
  NotificationSendRequest,
  NotificationXdgCategory,
} from "./ipc.ts";
import type { Unsubscribe } from "./transport.ts";

export type NotificationPermission = { allowed: boolean };
export type NotificationSent = { id: string };
export type NotificationCategoriesRegistered = { count: number };
export type NotificationRemoved = { count: number };

export type SendNotificationInput = {
  title: string;
  body?: string | null;
  subtitle?: string | null;
  image?: NotificationMediaReference | null;
  icon?: NotificationMediaReference | null;
  iconRoundCrop?: boolean;
  threadId?: string | null;
  categoryId?: string | null;
  userInfo?: Record<string, string>;
  xdgCategory?: NotificationXdgCategory | null;
};

export type NotificationsApi = {
  permissionState(): Promise<NotificationPermission>;
  requestPermission(): Promise<NotificationPermission>;
  capabilities(): Promise<NotificationCapabilities>;
  registerCategories(
    categories: NotificationCategory[],
  ): Promise<NotificationCategoriesRegistered>;
  send(input: SendNotificationInput): Promise<NotificationSent>;
  active(): Promise<ActiveNotification[]>;
  removeDelivered(ids: string[]): Promise<NotificationRemoved>;
  removeAllDelivered(): Promise<NotificationRemoved>;
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
  capabilities: async (): Promise<NotificationCapabilities> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "capabilities" },
    });
    return capabilitiesFromResult(result);
  },
  registerCategories: async (
    categories: NotificationCategory[],
  ): Promise<NotificationCategoriesRegistered> => {
    const result = await invokeNotification({
      command: "notification",
      payload: {
        notification: "registerCategories",
        payload: { categories },
      },
    });
    return categoriesRegisteredFromResult(result);
  },
  send: async (input: SendNotificationInput): Promise<NotificationSent> => {
    const payload: NotificationSendRequest = {
      title: input.title,
      body: input.body ?? null,
      subtitle: input.subtitle ?? null,
      image: input.image ?? null,
      icon: input.icon ?? null,
      iconRoundCrop: input.iconRoundCrop ?? false,
      threadId: input.threadId ?? null,
      categoryId: input.categoryId ?? null,
      userInfo: input.userInfo ?? {},
      xdgCategory: input.xdgCategory ?? null,
    };
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "send", payload },
    });
    if (result.result === "sent") return result.payload;
    return unsupportedNotificationResult(result);
  },
  active: async (): Promise<ActiveNotification[]> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "active" },
    });
    if (result.result === "active") return result.payload.notifications;
    return unexpectedNotificationResult(result, "active notifications");
  },
  removeDelivered: async (ids: string[]): Promise<NotificationRemoved> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "removeDelivered", payload: { ids } },
    });
    return removedFromResult(result);
  },
  removeAllDelivered: async (): Promise<NotificationRemoved> => {
    const result = await invokeNotification({
      command: "notification",
      payload: { notification: "removeAllDelivered" },
    });
    return removedFromResult(result);
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
    case "capabilities":
    case "categoriesRegistered":
    case "active":
    case "removed":
      throw new CefariError({
        code: "invalidCommand",
        details: {
          message: `unexpected notification result: ${result.result}`,
        },
      });
  }
}

function capabilitiesFromResult(
  result: NotificationResult,
): NotificationCapabilities {
  if (result.result === "capabilities") return result.payload;
  return unexpectedNotificationResult(result, "notification capabilities");
}

function categoriesRegisteredFromResult(
  result: NotificationResult,
): NotificationCategoriesRegistered {
  if (result.result === "categoriesRegistered") return result.payload;
  return unexpectedNotificationResult(result, "registered notification categories");
}

function removedFromResult(result: NotificationResult): NotificationRemoved {
  if (result.result === "removed") return result.payload;
  return unexpectedNotificationResult(result, "removed notifications");
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
  return unexpectedNotificationResult(result, "sent notification");
}

function unexpectedNotificationResult(
  result: NotificationResult,
  expected: string,
): never {
  throw new CefariError({
    code: "invalidCommand",
    details: {
      message: `expected ${expected}; received notification result ${result.result}`,
    },
  });
}
