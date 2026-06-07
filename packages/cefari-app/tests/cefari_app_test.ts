import {
  cefari,
  type CefariBridge,
  CefariError,
  type CefariIpcCommand,
  type CefariIpcEvent,
  type CefariIpcResponse,
} from "../src/mod.ts";

Deno.test("reports unavailable outside the Cefari shell", async () => {
  withBridge(undefined);

  assertEquals(cefari.isAvailable(), false);

  const result = await cefari.tryInvoke({ command: "updateState" });
  assertEquals(result.ok, false);
  if (!result.ok) {
    assertEquals(result.error.code, "unsupported");
  }

  await assertRejectsCefariError(
    () => cefari.updates.state(),
    "window.cefari is only available",
  );
});

Deno.test("wraps typed namespace commands", async () => {
  const commands: CefariIpcCommand[] = [];
  withBridge({
    invoke(command) {
      commands.push(command);
      return Promise.resolve(responseFor(command));
    },
    on() {
      return () => {};
    },
  });

  assertEquals(await cefari.window.focus(), {
    visible: true,
    focused: true,
    title: "Focused",
  });
  assertEquals(await cefari.window.setTitle("Dashboard"), {
    visible: true,
    focused: true,
    title: "Dashboard",
  });
  assertEquals(
    await cefari.shell.openExternalUrl(new URL("https://example.com")),
    {
      url: "https://example.com/",
    },
  );
  assertEquals(await cefari.updates.state(), { state: "current" });
  assertEquals(await cefari.updates.check(), {
    state: "available",
    version: "1.2.3",
  });
  assertEquals(await cefari.service.status(), { status: "running" });
  assertEquals(await cefari.tray.restoreWindow(), { restored: true });
  assertEquals(await cefari.notifications.permissionState(), { allowed: true });
  assertEquals(await cefari.notifications.requestPermission(), {
    allowed: true,
  });
  assertEquals(await cefari.notifications.send({ title: "Done" }), {
    id: "n1",
  });

  assertEquals(commands, [
    { command: "windowFocus" },
    { command: "windowSetTitle", payload: { title: "Dashboard" } },
    {
      command: "openExternalUrl",
      payload: { url: "https://example.com/" },
    },
    { command: "updateState" },
    { command: "updateCheck" },
    { command: "serviceStatus" },
    { command: "trayRestoreWindow" },
    {
      command: "notification",
      payload: { notification: "permissionState" },
    },
    {
      command: "notification",
      payload: { notification: "requestPermission" },
    },
    {
      command: "notification",
      payload: {
        notification: "send",
        payload: { title: "Done", body: null },
      },
    },
  ]);
});

Deno.test("filters typed events", () => {
  const handlers = new Set<(event: CefariIpcEvent) => void>();
  withBridge({
    invoke(command) {
      return Promise.resolve(responseFor(command));
    },
    on(handler) {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  });

  const focused: string[] = [];
  const notifications: string[] = [];

  const unsubscribeFocus = cefari.window.onFocused((state) => {
    focused.push(state.title);
  });
  const unsubscribeNotification = cefari.notifications.onResponse((event) => {
    notifications.push(event.id);
  });

  for (const handler of handlers) {
    handler({
      event: "windowFocused",
      payload: { visible: true, focused: true, title: "Dashboard" },
    });
    handler({
      event: "notification",
      payload: {
        event: "response",
        payload: { id: "n1", action: "default" },
      },
    });
  }

  assertEquals(focused, ["Dashboard"]);
  assertEquals(notifications, ["n1"]);

  unsubscribeFocus();
  unsubscribeNotification();
  assertEquals(handlers.size, 0);
});

Deno.test("throws typed errors for IPC failures", async () => {
  withBridge({
    invoke(command) {
      return Promise.resolve({
        id: "test",
        outcome: {
          status: "err",
          payload: {
            code: "denied",
            details: { message: `${command.command} denied` },
          },
        },
      });
    },
    on() {
      return () => {};
    },
  });

  const error = await assertRejectsCefariError(
    () => cefari.shell.openLogs(),
    "openLogs denied",
  );
  assertEquals(error.code, "denied");
  assertEquals(error.command, "openLogs");
});

function withBridge(bridge: CefariBridge | undefined) {
  const global = globalThis as { cefari?: CefariBridge };
  if (bridge) {
    global.cefari = bridge;
  } else {
    delete global.cefari;
  }
}

function responseFor(command: CefariIpcCommand): CefariIpcResponse {
  switch (command.command) {
    case "appQuit":
    case "openLogs":
      return ok({ result: "empty" });
    case "windowClose":
      return ok({
        result: "window",
        payload: { visible: false, focused: false, title: "Focused" },
      });
    case "windowSetTitle":
      return ok({
        result: "window",
        payload: { visible: true, focused: true, title: command.payload.title },
      });
    case "reloadUi":
      return ok({ result: "reloadUi" });
    case "windowShow":
    case "windowFocus":
      return ok({
        result: "window",
        payload: { visible: true, focused: true, title: "Focused" },
      });
    case "openExternalUrl":
      return ok({
        result: "externalUrl",
        payload: { url: command.payload.url },
      });
    case "updateState":
      return ok({ result: "updateState", payload: { state: "current" } });
    case "updateCheck":
      return ok({
        result: "updateCheck",
        payload: { state: "available", version: "1.2.3" },
      });
    case "serviceStatus":
      return ok({
        result: "serviceStatus",
        payload: { status: "running" },
      });
    case "trayRestoreWindow":
      return ok({ result: "tray", payload: { restored: true } });
    case "notification":
      switch (command.payload.notification) {
        case "permissionState":
          return ok({
            result: "notification",
            payload: { result: "permissionState", payload: { allowed: true } },
          });
        case "requestPermission":
          return ok({
            result: "notification",
            payload: {
              result: "permissionRequested",
              payload: { allowed: true },
            },
          });
        case "send":
          return ok({
            result: "notification",
            payload: { result: "sent", payload: { id: "n1" } },
          });
      }
  }
}

function ok(
  payload: Extract<CefariIpcResponse["outcome"], { status: "ok" }>["payload"],
): CefariIpcResponse {
  return {
    id: "test",
    outcome: { status: "ok", payload },
  };
}

function assert(
  condition: unknown,
  message = "assertion failed",
): asserts condition {
  if (!condition) throw new Error(message);
}

function assertEquals(actual: unknown, expected: unknown) {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) {
    throw new Error(
      `assertEquals failed\nactual: ${actualJson}\nexpected: ${expectedJson}`,
    );
  }
}

async function assertRejectsCefariError(
  fn: () => Promise<unknown>,
  messageIncludes: string,
): Promise<CefariError> {
  try {
    await fn();
  } catch (error) {
    assert(error instanceof CefariError, "expected CefariError");
    assert(
      error.message.includes(messageIncludes),
      `expected error message to include ${messageIncludes}, got ${error.message}`,
    );
    return error;
  }
  throw new Error("expected promise to reject");
}
