import {
  capabilitySupport,
  cefari,
  type CefariBridge,
  CefariError,
  type CefariIpcCommand,
  type CefariIpcEvent,
  type CefariIpcResponse,
  type NotificationResponseEvent,
  supportsTarget,
  type WindowKind,
  type WindowState,
} from "../../src/app/mod.ts";
import { daemonNativePath, getDaemonResources, isCefariDaemon } from "../../src/daemon.ts";

declare module "../../src/app/workers.ts" {
  interface CefariWorkerRegistry {
    thumbnailer: {
      init: { cacheDir: string };
      methods: {
        render: {
          input: { imageId: string };
          output: { ok: boolean };
          message: { progress: number };
        };
      };
    };
  }
}

Deno.test("reports unavailable outside the Cefari shell", async () => {
  withBridge(undefined);

  assertEquals(cefari.isAvailable(), false);
  assertEquals(cefari.desktop.daemon.isConfigured(), false);

  const result = await cefari.tryInvoke({ command: "updateState" });
  assertEquals(result.ok, false);
  if (!result.ok) {
    assertEquals(result.error.code, "unsupported");
  }

  await assertRejectsCefariError(
    () => cefari.desktop.updates.state(),
    "window.cefari is only available",
  );
  await assertRejectsCefariError(
    () => cefari.desktop.daemon.connect(),
    "daemon streams are only available",
  );
});

Deno.test("reports daemon helper availability outside Cefari daemon", () => {
  assertEquals(isCefariDaemon(), false);
});

Deno.test("exposes platform support metadata", () => {
  assertEquals(supportsTarget("windows", "desktop"), true);
  assertEquals(supportsTarget("windows", "ios"), false);
  assertEquals(supportsTarget("notifications", "android"), true);
  assertEquals(capabilitySupport("tray")?.support, "desktopOnly");
});

Deno.test("resolves daemon native resources from Cefari daemon env", () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "Deno");
  try {
    Object.defineProperty(globalThis, "Deno", {
      configurable: true,
      value: {
        env: {
          get(name: string) {
            return name === "CEFARI_DAEMON_RESOURCES"
              ? JSON.stringify({
                resourceDir: "/app/resources",
                nativeDir: "/app/resources/daemon/native",
                native: {
                  "thumb-tool": "/app/resources/daemon/native/bin/thumb",
                },
              })
              : undefined;
          },
        },
      },
    });

    assertEquals(getDaemonResources().nativeDir, "/app/resources/daemon/native");
    assertEquals(daemonNativePath("thumb-tool"), "/app/resources/daemon/native/bin/thumb");
  } finally {
    if (previous === undefined) {
      delete (globalThis as { Deno?: unknown }).Deno;
    } else {
      Object.defineProperty(globalThis, "Deno", previous);
    }
  }
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

  assertEquals(
    await cefari.desktop.windows.current(),
    mainWindowState("Focused"),
  );
  assertEquals(await cefari.desktop.windows.list(), [
    mainWindowState("Focused"),
  ]);
  assertEquals(
    await cefari.desktop.windows.get("main"),
    mainWindowState("Focused"),
  );
  assertEquals(
    await cefari.desktop.windows.create({
      id: "settings",
      route: "/settings",
      title: "Settings",
      width: 720,
      height: 560,
    }),
    mainWindowState("Settings", "settings", "secondary", "/settings"),
  );
  assertEquals(await cefari.desktop.window.focus(), {
    ...mainWindowState("Focused"),
    focused: true,
  });
  assertEquals(
    await cefari.desktop.windows.focus("settings"),
    mainWindowState("Focused"),
  );
  assertEquals(await cefari.desktop.window.setTitle("Dashboard"), {
    ...mainWindowState("Dashboard"),
    focused: true,
  });
  assertEquals(
    await cefari.desktop.windows.setTitle("settings", "Settings"),
    mainWindowState("Settings"),
  );
  assertEquals(
    await cefari.shell.openExternalUrl(new URL("https://example.com")),
    {
      url: "https://example.com/",
    },
  );
  assertEquals(await cefari.desktop.updates.state(), { state: "current" });
  assertEquals(await cefari.desktop.updates.check(), {
    state: "available",
    version: "1.2.3",
    updateId: "1.2.3",
  });
  assertEquals(await cefari.desktop.updates.apply({ updateId: "1.2.3" }), {
    state: "readyToRestart",
    version: "1.2.3",
    restartRequired: true,
  });
  await cefari.desktop.updates.restart();
  await cefari.desktop.updates.applyAndRestart();
  assertEquals(await cefari.desktop.service.status(), { status: "running" });
  assertEquals(await cefari.desktop.tray.restoreWindow(), { restored: true });
  assertEquals(await cefari.downloads.cancel("cef-1"), {
    result: "canceled",
    payload: { id: "cef-1" },
  });
  assertEquals(await cefari.downloads.reveal("cef-1"), {
    result: "revealed",
    payload: { id: "cef-1" },
  });
  const worker = await cefari.workers.spawn("thumbnailer", {
    cacheDir: "cache",
  });
  assertEquals(worker.id, "worker-1");
  assertEquals(worker.worker, "thumbnailer");
  assertEquals(worker.status, "running");
  assertEquals(await worker.invoke("render", { imageId: "abc" }), {
    ok: true,
  });
  assertEquals(await cefari.workers.list(), [
    { id: "worker-1", worker: "thumbnailer", status: "running" },
  ]);
  await worker.terminate();
  assertEquals(await cefari.notifications.permissionState(), { allowed: true });
  assertEquals(await cefari.notifications.requestPermission(), {
    allowed: true,
  });
  assertEquals(
    await cefari.notifications.capabilities(),
    notificationCapabilities(),
  );
  assertEquals(
    await cefari.notifications.registerCategories([
      {
        id: "message",
        actions: [
          { type: "action", id: "open", title: "Open" },
          {
            type: "textInput",
            id: "reply",
            title: "Reply",
            inputButtonTitle: "Send",
            inputPlaceholder: "Message",
          },
        ],
      },
    ]),
    { count: 1 },
  );
  assertEquals(await cefari.notifications.send({ title: "Done" }), {
    id: "n1",
  });
  assertEquals(
    await cefari.notifications.send({
      title: "Build complete",
      body: "The package is ready.",
      subtitle: "Release",
      image: { source: "appResource", path: "images/build.png" },
      icon: { source: "appData", path: "icons/build.png" },
      iconRoundCrop: true,
      threadId: "builds",
      categoryId: "message",
      userInfo: { buildId: "123" },
      xdgCategory: "transferComplete",
    }),
    { id: "n1" },
  );
  assertEquals(await cefari.notifications.active(), [
    { id: "n1", userInfo: { buildId: "123" } },
  ]);
  assertEquals(await cefari.notifications.removeDelivered(["n1"]), {
    count: 1,
  });
  assertEquals(await cefari.notifications.removeAllDelivered(), {
    count: 2,
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
  assertEquals(await cefari.files.exists("state.json"), true);
  assertEquals(
    await cefari.files.readBytes("blob.bin"),
    new Uint8Array([1, 2]),
  );

  assertEquals(commands, [
    { command: "windowCurrent" },
    { command: "windowList" },
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
    { command: "windowFocus", payload: { target: { id: "settings" } } },
    {
      command: "windowSetTitle",
      payload: { target: null, title: "Dashboard" },
    },
    {
      command: "windowSetTitle",
      payload: { target: { id: "settings" }, title: "Settings" },
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
      command: "worker",
      payload: {
        worker: "spawn",
        payload: {
          worker: "thumbnailer",
          inputJson: '{"cacheDir":"cache"}',
        },
      },
    },
    {
      command: "worker",
      payload: {
        worker: "invoke",
        payload: {
          id: "worker-1",
          method: "render",
          inputJson: '{"imageId":"abc"}',
        },
      },
    },
    {
      command: "worker",
      payload: { worker: "list" },
    },
    {
      command: "worker",
      payload: {
        worker: "terminate",
        payload: { id: "worker-1" },
      },
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
      payload: { notification: "capabilities" },
    },
    {
      command: "notification",
      payload: {
        notification: "registerCategories",
        payload: {
          categories: [
            {
              id: "message",
              actions: [
                { type: "action", id: "open", title: "Open" },
                {
                  type: "textInput",
                  id: "reply",
                  title: "Reply",
                  inputButtonTitle: "Send",
                  inputPlaceholder: "Message",
                },
              ],
            },
          ],
        },
      },
    },
    {
      command: "notification",
      payload: {
        notification: "send",
        payload: {
          title: "Done",
          body: null,
          subtitle: null,
          image: null,
          icon: null,
          iconRoundCrop: false,
          threadId: null,
          categoryId: null,
          userInfo: {},
          xdgCategory: null,
        },
      },
    },
    {
      command: "notification",
      payload: {
        notification: "send",
        payload: {
          title: "Build complete",
          body: "The package is ready.",
          subtitle: "Release",
          image: { source: "appResource", path: "images/build.png" },
          icon: { source: "appData", path: "icons/build.png" },
          iconRoundCrop: true,
          threadId: "builds",
          categoryId: "message",
          userInfo: { buildId: "123" },
          xdgCategory: "transferComplete",
        },
      },
    },
    {
      command: "notification",
      payload: { notification: "active" },
    },
    {
      command: "notification",
      payload: { notification: "removeDelivered", payload: { ids: ["n1"] } },
    },
    {
      command: "notification",
      payload: { notification: "removeAllDelivered" },
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
      payload: { file: "exists", payload: { path: "state.json" } },
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
  const filteredFocus: string[] = [];
  const notifications: Array<
    [
      string,
      NotificationResponseEvent["action"],
      string | null,
      Record<string, string>,
    ]
  > = [];
  const workerMessages: number[] = [];

  const unsubscribeFocus = cefari.desktop.window.onFocused((state) => {
    focused.push(state.title);
  });
  const unsubscribeDeepLink = cefari.on("deepLinkOpened", (event) => {
    deepLinks.push(event.url);
  });
  const unsubscribeDownload = cefari.on("download.completed", (event) => {
    downloads.push(event.destinationPath);
  });
  const unsubscribeFilteredFocus = cefari.desktop.windows.onFocused(
    (event) => filteredFocus.push(event.windowId),
    { windowId: "settings" },
  );
  const unsubscribeNotification = cefari.notifications.onResponse((event) => {
    notifications.push([
      event.id,
      event.action,
      event.userText,
      event.userInfo,
    ]);
  });
  const unsubscribeWorker = cefari.on("worker.message", (event) => {
    if (event.id === "worker-1") {
      workerMessages.push(JSON.parse(event.messageJson).progress);
    }
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
      event: "windowFocused",
      payload: {
        windowId: "settings",
        state: mainWindowState(
          "Settings",
          "settings",
          "secondary",
          "/settings",
        ),
      },
    });
    handler({
      event: "notification",
      payload: {
        event: "response",
        payload: {
          id: "n1",
          action: "default",
          userText: null,
          userInfo: { buildId: "123" },
        },
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
    handler({
      event: "worker",
      payload: {
        event: "message",
        payload: {
          id: "worker-1",
          worker: "thumbnailer",
          requestId: "request-1",
          method: "render",
          messageJson: '{"progress":0.5}',
        },
      },
    });
  }

  assertEquals(focused, ["Dashboard", "Settings"]);
  assertEquals(deepLinks, ["myapp://open/item?id=1"]);
  assertEquals(downloads, ["/tmp/file.txt"]);
  assertEquals(filteredFocus, ["settings"]);
  assertEquals(notifications, [["n1", "default", null, { buildId: "123" }]]);
  assertEquals(workerMessages, [0.5]);

  unsubscribeFocus();
  unsubscribeDeepLink();
  unsubscribeDownload();
  unsubscribeFilteredFocus();
  unsubscribeNotification();
  unsubscribeWorker();
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

Deno.test("throws typed errors for unconfigured daemon bridge", async () => {
  withDaemonBridge({
    post() {
      return Promise.resolve({
        outcome: {
          status: "err",
          payload: {
            code: "unsupported",
            details: {
              command: "daemon",
              reason: "daemon is not configured",
            },
          },
        },
      });
    },
    on() {
      return () => {};
    },
  });

  assertEquals(cefari.desktop.daemon.isConfigured(), true);
  const error = await assertRejectsCefariError(
    () => cefari.desktop.daemon.connect(),
    "daemon is not configured",
  );
  assertEquals(error.code, "unsupported");
  assertEquals(error.command, "daemon");
});

Deno.test("plumbs daemon stream connect, read, write, and close", async () => {
  const commands: unknown[] = [];
  const handlers = new Set<(event: DaemonBridgeEvent) => void>();
  withDaemonBridge({
    post(command) {
      commands.push(command);
      if (command.op === "connect") {
        return Promise.resolve(daemonOk({ connectionId: 7 }));
      }
      return Promise.resolve(daemonOk({}));
    },
    on(handler) {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  });

  const connection = await cefari.desktop.daemon.connect();
  assertEquals(cefari.desktop.daemon.isConfigured(), true);

  const reader = connection.readable.getReader();
  for (const handler of handlers) {
    handler({ event: "chunk", connectionId: 7, chunkBase64: "AQI=" });
  }
  const readResult = await reader.read();
  assertEquals(readResult.done, false);
  assertEquals(readResult.value, new Uint8Array([1, 2]));

  const writer = connection.writable.getWriter();
  await writer.write(new Uint8Array([3, 4]));
  await writer.close();
  writer.releaseLock();
  reader.releaseLock();
  await connection.close();
  await connection.closed;

  assertEquals(commands, [
    { op: "connect" },
    { op: "write", connectionId: 7, chunkBase64: "AwQ=" },
    { op: "closeWrite", connectionId: 7 },
    { op: "close", connectionId: 7 },
  ]);
  assertEquals(handlers.size, 0);
});

function withBridge(bridge: CefariBridge | undefined) {
  const global = globalThis as {
    cefari?: CefariBridge;
    __CEFARI_DAEMON_STREAM_POST__?: DaemonBridgeHooks["post"];
    __CEFARI_DAEMON_STREAM_ON__?: DaemonBridgeHooks["on"];
  };
  if (bridge) {
    global.cefari = bridge;
  } else {
    delete global.cefari;
  }
  delete global.__CEFARI_DAEMON_STREAM_POST__;
  delete global.__CEFARI_DAEMON_STREAM_ON__;
}

type DaemonBridgeCommand =
  | { op: "connect" }
  | { op: "write"; connectionId: number; chunkBase64: string }
  | { op: "closeWrite"; connectionId: number }
  | { op: "close"; connectionId: number };

type DaemonBridgeEvent =
  | { event: "chunk"; connectionId: number; chunkBase64: string }
  | { event: "close"; connectionId: number }
  | { event: "error"; connectionId: number; message: string };

type DaemonBridgeResponse = {
  outcome:
    | { status: "ok"; payload: { connectionId?: number } }
    | {
      status: "err";
      payload: Extract<
        CefariIpcResponse["outcome"],
        { status: "err" }
      >["payload"];
    };
};

type DaemonBridgeHooks = {
  post(command: DaemonBridgeCommand): Promise<DaemonBridgeResponse>;
  on(handler: (event: DaemonBridgeEvent) => void): () => void;
};

function withDaemonBridge(hooks: DaemonBridgeHooks) {
  withBridge(undefined);
  const global = globalThis as {
    __CEFARI_DAEMON_STREAM_POST__?: DaemonBridgeHooks["post"];
    __CEFARI_DAEMON_STREAM_ON__?: DaemonBridgeHooks["on"];
  };
  global.__CEFARI_DAEMON_STREAM_POST__ = hooks.post;
  global.__CEFARI_DAEMON_STREAM_ON__ = hooks.on;
}

function daemonOk(payload: { connectionId?: number }): DaemonBridgeResponse {
  return {
    outcome: { status: "ok", payload },
  };
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
    case "worker":
      switch (command.payload.worker) {
        case "spawn":
          return ok({
            result: "worker",
            payload: {
              result: "spawned",
              payload: {
                id: "worker-1",
                worker: command.payload.payload.worker,
                status: "running",
              },
            },
          });
        case "invoke":
          return ok({
            result: "worker",
            payload: {
              result: "invoked",
              payload: {
                id: command.payload.payload.id,
                method: command.payload.payload.method,
                outputJson: '{"ok":true}',
              },
            },
          });
        case "terminate":
          return ok({
            result: "worker",
            payload: {
              result: "terminated",
              payload: { id: command.payload.payload.id },
            },
          });
        case "list":
          return ok({
            result: "worker",
            payload: {
              result: "list",
              payload: {
                workers: [
                  {
                    id: "worker-1",
                    worker: "thumbnailer",
                    status: "running",
                  },
                ],
              },
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
        case "capabilities":
          return ok({
            result: "notification",
            payload: {
              result: "capabilities",
              payload: notificationCapabilities(),
            },
          });
        case "registerCategories":
          return ok({
            result: "notification",
            payload: {
              result: "categoriesRegistered",
              payload: { count: command.payload.payload.categories.length },
            },
          });
        case "send":
          return ok({
            result: "notification",
            payload: { result: "sent", payload: { id: "n1" } },
          });
        case "active":
          return ok({
            result: "notification",
            payload: {
              result: "active",
              payload: {
                notifications: [
                  { id: "n1", userInfo: { buildId: "123" } },
                ],
              },
            },
          });
        case "removeDelivered":
          return ok({
            result: "notification",
            payload: {
              result: "removed",
              payload: { count: command.payload.payload.ids.length },
            },
          });
        case "removeAllDelivered":
          return ok({
            result: "notification",
            payload: { result: "removed", payload: { count: 2 } },
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
        case "exists":
          return ok({
            result: "file",
            payload: { result: "exists", payload: { exists: true } },
          });
      }
  }
}

function notificationCapabilities() {
  return {
    permissionState: true,
    permissionPrompt: true,
    subtitle: true,
    image: true,
    icon: true,
    iconRoundCrop: true,
    threadId: true,
    categories: true,
    actionButtons: true,
    textInputActions: true,
    userInfo: true,
    xdgCategory: true,
    activeNotifications: true,
    removeDelivered: true,
    responseEvents: true,
    coldStartActivation: true,
  };
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
