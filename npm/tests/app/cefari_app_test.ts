import {
  cefari,
  type CefariBridge,
  CefariError,
  type CefariIpcCommand,
  type CefariIpcEvent,
  type CefariIpcResponse,
  type WindowKind,
  type WindowState,
} from "../../src/app/mod.ts";

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

  assertEquals(await cefari.window.current(), mainWindowState("Focused"));
  assertEquals(await cefari.window.list(), [mainWindowState("Focused")]);
  assertEquals(
    await cefari.window.create({
      id: "settings",
      route: "/settings",
      title: "Settings",
      width: 720,
      height: 560,
    }),
    mainWindowState("Settings", "settings", "secondary", "/settings"),
  );
  assertEquals(await cefari.window.focus(), {
    ...mainWindowState("Focused"),
    focused: true,
  });
  assertEquals(await cefari.window.setTitle("Dashboard"), {
    ...mainWindowState("Dashboard"),
    focused: true,
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
  assertEquals(await cefari.downloads.cancel("cef-1"), {
    result: "canceled",
    payload: { id: "cef-1" },
  });
  assertEquals(await cefari.downloads.reveal("cef-1"), {
    result: "revealed",
    payload: { id: "cef-1" },
  });
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
    { command: "windowCurrent" },
    { command: "windowList" },
    {
      command: "windowCreate",
      payload: {
        id: "settings",
        route: "/settings",
        title: "Settings",
        width: 720,
        height: 560,
        minWidth: null,
        minHeight: null,
        maxWidth: null,
        maxHeight: null,
        x: null,
        y: null,
        visible: null,
        focused: null,
        resizable: null,
        decorations: null,
        alwaysOnTop: null,
        parentId: null,
        modal: null,
        persistKey: null,
      },
    },
    { command: "windowFocus", payload: { target: null } },
    {
      command: "windowSetTitle",
      payload: { target: null, title: "Dashboard" },
    },
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
      command: "download",
      payload: { download: "cancel", payload: { id: "cef-1" } },
    },
    {
      command: "download",
      payload: { download: "reveal", payload: { id: "cef-1" } },
    },
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

Deno.test("wraps native dialog commands and cancellation", async () => {
  const commands: CefariIpcCommand[] = [];
  withBridge({
    invoke(command) {
      commands.push(command);
      if (
        command.command === "dialog" &&
        command.payload.dialog === "chooseFolders"
      ) {
        return Promise.resolve(ok({
          result: "dialog",
          payload: { result: "canceled" },
        }));
      }
      return Promise.resolve(responseFor(command));
    },
    on() {
      return () => {};
    },
  });

  const options = {
    title: "Choose project asset",
    filters: [{ name: "Images", extensions: ["png", "jpg"] }],
    defaultDirectory: { kind: "appData" as const, path: "exports" },
    defaultName: "report.png",
    modality: "window" as const,
    canCreateDirectories: true,
  };

  assertEquals(await cefari.dialogs.openFile(options), {
    canceled: false,
    value: {
      path: "/tmp/report.png",
      name: "report.png",
      kind: "file",
    },
  });
  assertEquals(await cefari.dialogs.openFiles(), {
    canceled: false,
    value: [
      {
        path: "/tmp/report.png",
        name: "report.png",
        kind: "file",
      },
    ],
  });
  assertEquals(await cefari.dialogs.chooseFolder(), {
    canceled: false,
    value: {
      path: "/tmp/projects",
      name: "projects",
      kind: "directory",
    },
  });
  assertEquals(await cefari.dialogs.chooseFolders(), {
    canceled: true,
  });
  assertEquals(await cefari.dialogs.saveFile({ defaultName: "saved.txt" }), {
    canceled: false,
    value: {
      path: "/tmp/saved.txt",
      name: "saved.txt",
      kind: "file",
    },
  });

  assertEquals(commands, [
    {
      command: "dialog",
      payload: {
        dialog: "openFile",
        payload: {
          title: "Choose project asset",
          filters: [{ name: "Images", extensions: ["png", "jpg"] }],
          defaultDirectory: { kind: "appData", path: "exports" },
          defaultName: "report.png",
          modality: "window",
          canCreateDirectories: true,
        },
      },
    },
    {
      command: "dialog",
      payload: {
        dialog: "openFiles",
        payload: {
          title: null,
          filters: [],
          defaultDirectory: null,
          defaultName: null,
          modality: "window",
          canCreateDirectories: null,
        },
      },
    },
    {
      command: "dialog",
      payload: {
        dialog: "chooseFolder",
        payload: {
          title: null,
          filters: [],
          defaultDirectory: null,
          defaultName: null,
          modality: "window",
          canCreateDirectories: null,
        },
      },
    },
    {
      command: "dialog",
      payload: {
        dialog: "chooseFolders",
        payload: {
          title: null,
          filters: [],
          defaultDirectory: null,
          defaultName: null,
          modality: "window",
          canCreateDirectories: null,
        },
      },
    },
    {
      command: "dialog",
      payload: {
        dialog: "saveFile",
        payload: {
          title: null,
          filters: [],
          defaultDirectory: null,
          defaultName: "saved.txt",
          modality: "window",
          canCreateDirectories: null,
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
  const deepLinks: string[] = [];
  const downloads: string[] = [];
  const notifications: string[] = [];

  const unsubscribeFocus = cefari.window.onFocused((state) => {
    focused.push(state.title);
  });
  const unsubscribeDeepLink = cefari.on("deepLinkOpened", (event) => {
    deepLinks.push(event.url);
  });
  const unsubscribeDownload = cefari.on("download.completed", (event) => {
    downloads.push(event.destinationPath);
  });
  const unsubscribeNotification = cefari.notifications.onResponse((event) => {
    notifications.push(event.id);
  });

  for (const handler of handlers) {
    handler({
      event: "windowFocused",
      payload: {
        windowId: "main",
        state: mainWindowState("Dashboard"),
      },
    });
    handler({
      event: "notification",
      payload: {
        event: "response",
        payload: { id: "n1", action: "default" },
      },
    });
    handler({
      event: "deepLinkOpened",
      payload: { url: "myapp://open/item?id=1" },
    });
    handler({
      event: "download",
      payload: {
        event: "completed",
        payload: {
          id: "cef-1",
          url: "https://example.test/file.txt",
          destinationPath: "/tmp/file.txt",
          receivedBytes: 10,
          totalBytes: 10,
        },
      },
    });
  }

  assertEquals(focused, ["Dashboard"]);
  assertEquals(deepLinks, ["myapp://open/item?id=1"]);
  assertEquals(downloads, ["/tmp/file.txt"]);
  assertEquals(notifications, ["n1"]);

  unsubscribeFocus();
  unsubscribeDeepLink();
  unsubscribeDownload();
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

function mainWindowState(
  title: string,
  id = "main",
  kind: WindowKind = "main",
  route: string | null = null,
): WindowState {
  return {
    id,
    kind,
    visible: true,
    focused: true,
    title,
    modal: false,
    parentId: null,
    route,
  };
}

function responseFor(command: CefariIpcCommand): CefariIpcResponse {
  switch (command.command) {
    case "appQuit":
    case "openLogs":
      return ok({ result: "empty" });
    case "windowCurrent":
      return ok({
        result: "window",
        payload: mainWindowState("Focused"),
      });
    case "windowList":
      return ok({
        result: "windowList",
        payload: { windows: [mainWindowState("Focused")] },
      });
    case "windowCreate":
      return ok({
        result: "window",
        payload: mainWindowState(
          command.payload.title ?? "Focused",
          command.payload.id ?? "secondary",
          "secondary",
          command.payload.route,
        ),
      });
    case "windowClose":
      return ok({
        result: "window",
        payload: {
          ...mainWindowState("Focused"),
          visible: false,
          focused: false,
        },
      });
    case "windowSetTitle":
      return ok({
        result: "window",
        payload: mainWindowState(command.payload.title),
      });
    case "reloadUi":
      return ok({ result: "reloadUi" });
    case "windowShow":
    case "windowFocus":
      return ok({
        result: "window",
        payload: mainWindowState("Focused"),
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
    case "download":
      switch (command.payload.download) {
        case "cancel":
          return ok({
            result: "download",
            payload: {
              result: "canceled",
              payload: { id: command.payload.payload.id },
            },
          });
        case "reveal":
          return ok({
            result: "download",
            payload: {
              result: "revealed",
              payload: { id: command.payload.payload.id },
            },
          });
      }
      break;
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
    case "dialog":
      switch (command.payload.dialog) {
        case "openFile":
        case "openFiles":
          return ok({
            result: "dialog",
            payload: {
              result: "selected",
              payload: {
                paths: [
                  {
                    path: "/tmp/report.png",
                    name: "report.png",
                    kind: "file",
                  },
                ],
              },
            },
          });
        case "chooseFolder":
        case "chooseFolders":
          return ok({
            result: "dialog",
            payload: {
              result: "selected",
              payload: {
                paths: [
                  {
                    path: "/tmp/projects",
                    name: "projects",
                    kind: "directory",
                  },
                ],
              },
            },
          });
        case "saveFile":
          return ok({
            result: "dialog",
            payload: {
              result: "selected",
              payload: {
                paths: [
                  {
                    path: "/tmp/saved.txt",
                    name: "saved.txt",
                    kind: "file",
                  },
                ],
              },
            },
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
