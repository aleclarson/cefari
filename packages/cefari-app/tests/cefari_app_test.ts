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
    updateId: "1.2.3",
  });
  assertEquals(await cefari.updates.apply({ updateId: "1.2.3" }), {
    state: "readyToRestart",
    version: "1.2.3",
    restartRequired: true,
  });
  await cefari.updates.restart();
  await cefari.updates.applyAndRestart();
  assertEquals(await cefari.service.status(), { status: "running" });
  assertEquals(await cefari.tray.restoreWindow(), { restored: true });
  assertEquals(await cefari.notifications.permissionState(), { allowed: true });
  assertEquals(await cefari.notifications.requestPermission(), {
    allowed: true,
  });
  assertEquals(await cefari.notifications.send({ title: "Done" }), {
    id: "n1",
  });
  assertEquals(await cefari.fs.readFile("state.json", "utf8"), "{}");
  await cefari.fs.writeFile("state.json", "{}");
  assertEquals(await cefari.fs.readdir(".", { withFileTypes: true }), [
    {
      name: "state.json",
      path: "state.json",
      kind: "file",
    },
  ]);
  assertEquals(await cefari.fs.access("state.json"), true);
  assertEquals(await cefari.files.appDataDir(), {
    rootKind: "appData",
    displayPath: "/tmp/cefari",
  });
  assertEquals(
    await cefari.files.readBytes("blob.bin"),
    new Uint8Array([1, 2]),
  );

  assertEquals(commands, [
    { command: "windowFocus" },
    { command: "windowSetTitle", payload: { title: "Dashboard" } },
    {
      command: "openExternalUrl",
      payload: { url: "https://example.com/" },
    },
    { command: "updateState" },
    { command: "updateCheck" },
    { command: "updateApply", payload: { updateId: "1.2.3" } },
    { command: "updateRestart" },
    { command: "updateApply", payload: { updateId: null } },
    { command: "updateRestart" },
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
    {
      command: "files",
      payload: {
        file: "readFile",
        payload: { path: "state.json", encoding: "utf8" },
      },
    },
    {
      command: "files",
      payload: {
        file: "writeFile",
        payload: {
          path: "state.json",
          contents: { kind: "text", value: "{}" },
          options: { createParents: true, overwrite: true },
        },
      },
    },
    {
      command: "files",
      payload: {
        file: "readdir",
        payload: { path: ".", withFileTypes: true },
      },
    },
    {
      command: "files",
      payload: { file: "access", payload: { path: "state.json" } },
    },
    {
      command: "files",
      payload: { file: "appDataDir" },
    },
    {
      command: "files",
      payload: {
        file: "readFile",
        payload: { path: "blob.bin", encoding: "base64" },
      },
    },
  ]);
});

Deno.test("normalizes fs text, bytes, and base64 encodings", async () => {
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

  assertEquals(await cefari.fs.readFile("blob.bin"), new Uint8Array([1, 2]));
  await cefari.fs.writeFile("blob.bin", new Uint8Array([1, 2]));
  await cefari.fs.writeFile("encoded.txt", "AQI=", { encoding: "base64" });

  assertEquals(commands, [
    {
      command: "files",
      payload: {
        file: "readFile",
        payload: { path: "blob.bin", encoding: "base64" },
      },
    },
    {
      command: "files",
      payload: {
        file: "writeFile",
        payload: {
          path: "blob.bin",
          contents: { kind: "base64", value: "AQI=" },
          options: { createParents: true, overwrite: true },
        },
      },
    },
    {
      command: "files",
      payload: {
        file: "writeFile",
        payload: {
          path: "encoded.txt",
          contents: { kind: "base64", value: "AQI=" },
          options: { createParents: true, overwrite: true },
        },
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
        payload: { state: "available", version: "1.2.3", updateId: "1.2.3" },
      });
    case "updateApply":
      return ok({
        result: "updateApply",
        payload: {
          state: "readyToRestart",
          version: "1.2.3",
          restartRequired: true,
        },
      });
    case "updateRestart":
      return ok({ result: "empty" });
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
      break;
    case "files":
      switch (command.payload.file) {
        case "appDataDir":
          return ok({
            result: "file",
            payload: {
              result: "appDataDir",
              payload: { rootKind: "appData", displayPath: "/tmp/cefari" },
            },
          });
        case "readFile":
          if (command.payload.payload.encoding === "utf8") {
            return ok({
              result: "file",
              payload: { result: "text", payload: { contents: "{}" } },
            });
          }
          return ok({
            result: "file",
            payload: { result: "base64", payload: { contents: "AQI=" } },
          });
        case "writeFile":
          return ok({
            result: "file",
            payload: {
              result: "written",
              payload: {
                path: command.payload.payload.path,
                bytesWritten: 2,
              },
            },
          });
        case "readdir":
          return ok({
            result: "file",
            payload: {
              result: "dirEntries",
              payload: {
                entries: [
                  {
                    kind: "file",
                    name: "state.json",
                    path: "state.json",
                  },
                ],
              },
            },
          });
        case "mkdir":
        case "rm":
        case "rename":
        case "copyFile":
          return ok({
            result: "file",
            payload: { result: "empty" },
          });
        case "stat":
          return ok({
            result: "file",
            payload: {
              result: "stat",
              payload: {
                path: command.payload.payload.path,
                kind: "file",
                size: 2,
                modifiedAtMs: null,
                createdAtMs: null,
              },
            },
          });
        case "access":
          return ok({
            result: "file",
            payload: { result: "access", payload: { ok: true } },
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
