import { cefari } from "cefari/app";

const statusElement = document.getElementById("status");
const phaseKey = "cefari.smoke.phase";
const resultPath = "smoke/result.json";
const workRoot = "smoke/work";
const steps = [];

function record(name, detail = "") {
  const line = `${new Date().toISOString()} ${name}${
    detail ? `: ${detail}` : ""
  }`;
  steps.push(line);
  statusElement.textContent = steps.join("\n");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function commandFile(file, payload) {
  return payload === undefined
    ? { command: "files", payload: { file } }
    : { command: "files", payload: { file, payload } };
}

async function invoke(command) {
  record("invoke", command.command);
  const response = await window.cefari.invoke(command);
  record(`${command.command} response`, response.outcome.status);
  return response;
}

async function invokeOk(command, resultTag) {
  const response = await invoke(command);
  if (response.outcome.status !== "ok") {
    throw new Error(
      `${command.command} failed: ${JSON.stringify(response.outcome.payload)}`,
    );
  }

  const result = response.outcome.payload;
  if (resultTag && result.result !== resultTag) {
    throw new Error(
      `${command.command} returned ${result.result}, expected ${resultTag}`,
    );
  }
  return result;
}

async function expectError(command, code) {
  const response = await invoke(command);
  if (response.outcome.status !== "err") {
    throw new Error(`${command.command} unexpectedly succeeded`);
  }
  if (response.outcome.payload.code !== code) {
    throw new Error(
      `${command.command} returned ${response.outcome.payload.code}, expected ${code}`,
    );
  }
  return response.outcome.payload;
}

async function expectFileEmpty(command) {
  const result = await invokeOk(command, "file");
  assert(
    result.payload.result === "empty" || result.payload.result === "written",
    `unexpected file result ${result.payload.result}`,
  );
  return result.payload;
}

function commandWorker(worker, payload) {
  return payload === undefined
    ? { command: "worker", payload: { worker } }
    : { command: "worker", payload: { worker, payload } };
}

async function invokeWorkerOk(worker, payload, resultTag) {
  const result = await invokeOk(commandWorker(worker, payload), "worker");
  if (result.payload.result !== resultTag) {
    throw new Error(
      `worker command returned ${result.payload.result}, expected ${resultTag}`,
    );
  }
  return result.payload.payload;
}

function waitForWorkerEvent(id, eventName, predicate = () => true, timeoutMs = 10_000) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      unsubscribe();
      reject(new Error(`timed out waiting for ${eventName} from worker ${id}`));
    }, timeoutMs);
    const unsubscribe = window.cefari.on((event) => {
      if (event.event !== eventName || event.payload.id !== id) return;
      if (!predicate(event.payload)) return;
      clearTimeout(timeout);
      unsubscribe();
      resolve(event.payload);
    });
  });
}

function parseWorkerMessage(event) {
  return JSON.parse(event.messageJson);
}

async function writeResult(status, details = {}) {
  const payload = {
    status,
    location: window.location.href,
    completedAt: new Date().toISOString(),
    steps,
    ...details,
  };
  await expectFileEmpty(commandFile("writeFile", {
    path: resultPath,
    contents: { kind: "text", value: `${JSON.stringify(payload, null, 2)}\n` },
    options: { createParents: true, overwrite: true },
  }));
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function daemonRoundTripSmoke() {
  record("daemon", "connect");
  assert(
    cefari.daemon.isConfigured(),
    "daemon stream bridge is not configured",
  );

  const connection = await cefari.daemon.connect();
  const writer = connection.writable.getWriter();
  const reader = connection.readable.getReader();
  const input = new Uint8Array([67, 101, 102, 97, 114, 105]);

  await writer.write(input);
  const output = await reader.read();
  assert(output.done === false, "daemon closed before echoing bytes");
  assert(output.value.length === input.length, "daemon echo length changed");
  for (let index = 0; index < input.length; index += 1) {
    assert(output.value[index] === input[index], "daemon echo bytes changed");
  }

  reader.releaseLock();
  await writer.close();
  writer.releaseLock();
  await connection.close();
  await connection.closed;
  record("daemon", "round-trip ok");
}

async function preReloadSmoke() {
  record("preflight", window.location.href);
  assert(
    window.location.protocol === "http:",
    "cefari dev should load the static dev server",
  );
  assert(
    window.location.hostname === "127.0.0.1" ||
      window.location.hostname === "localhost",
    `unexpected dev hostname ${window.location.hostname}`,
  );
  assert(
    window.cefari && typeof window.cefari.invoke === "function",
    "window.cefari is missing",
  );
  assert(typeof window.cefari.on === "function", "window.cefari.on is missing");

  const unsubscribe = window.cefari.on(() => {});
  assert(
    typeof unsubscribe === "function",
    "window.cefari.on did not return an unsubscribe",
  );
  unsubscribe();

  const defaultStyles = document.getElementById("cefari-default-styles");
  assert(defaultStyles, "default Cefari drag-region styles were not injected");
  assert(
    defaultStyles.textContent.includes(".cefari-drag"),
    "drag-region CSS is missing",
  );

  await expectError({ command: "definitelyUnknownCommand" }, "unknownCommand");

  sessionStorage.setItem(phaseKey, "after-reload");
  await invokeOk({ command: "reloadUi" }, "reloadUi");
  await delay(1_500);
  throw new Error("reloadUi returned but the frontend was not reloaded");
}

async function fileSmoke() {
  const appData = await invokeOk(commandFile("appDataDir"), "file");
  assert(
    appData.payload.result === "appDataDir",
    "appDataDir returned the wrong result",
  );
  assert(
    appData.payload.payload.displayPath,
    "appDataDir did not include a display path",
  );

  await expectFileEmpty(commandFile("rm", {
    path: workRoot,
    recursive: true,
    force: true,
  }));
  await expectFileEmpty(commandFile("mkdir", {
    path: `${workRoot}/nested`,
    recursive: true,
  }));
  await expectFileEmpty(commandFile("writeFile", {
    path: `${workRoot}/nested/input.txt`,
    contents: { kind: "text", value: "cefari smoke text\n" },
    options: { createParents: true, overwrite: true },
  }));

  const text = await invokeOk(
    commandFile("readFile", {
      path: `${workRoot}/nested/input.txt`,
      encoding: "utf8",
    }),
    "file",
  );
  assert(text.payload.result === "text", "text read returned the wrong result");
  assert(
    text.payload.payload.contents === "cefari smoke text\n",
    "text contents changed",
  );

  await expectFileEmpty(commandFile("writeFile", {
    path: `${workRoot}/nested/bytes.bin`,
    contents: { kind: "base64", value: "AQIDBA==" },
    options: { createParents: true, overwrite: true },
  }));
  const bytes = await invokeOk(
    commandFile("readFile", {
      path: `${workRoot}/nested/bytes.bin`,
      encoding: "base64",
    }),
    "file",
  );
  assert(
    bytes.payload.result === "base64",
    "byte read returned the wrong result",
  );
  assert(
    bytes.payload.payload.contents === "AQIDBA==",
    "byte contents changed",
  );

  const entries = await invokeOk(
    commandFile("readdir", {
      path: `${workRoot}/nested`,
      withFileTypes: true,
    }),
    "file",
  );
  assert(
    entries.payload.result === "dirEntries",
    "readdir returned the wrong result",
  );
  assert(
    entries.payload.payload.entries.some((entry) =>
      entry.name === "input.txt" && entry.kind === "file"
    ),
    "readdir did not include input.txt",
  );

  const stat = await invokeOk(
    commandFile("stat", {
      path: `${workRoot}/nested/input.txt`,
    }),
    "file",
  );
  assert(stat.payload.result === "stat", "stat returned the wrong result");
  assert(stat.payload.payload.kind === "file", "stat did not identify a file");
  assert(stat.payload.payload.size > 0, "stat size was not populated");

  const accessOk = await invokeOk(
    commandFile("access", {
      path: `${workRoot}/nested/input.txt`,
    }),
    "file",
  );
  assert(
    accessOk.payload.result === "access",
    "access returned the wrong result",
  );
  assert(
    accessOk.payload.payload.ok === true,
    "access did not find an existing file",
  );

  const accessMissing = await invokeOk(
    commandFile("access", {
      path: `${workRoot}/missing.txt`,
    }),
    "file",
  );
  assert(
    accessMissing.payload.result === "access",
    "missing access returned the wrong result",
  );
  assert(
    accessMissing.payload.payload.ok === false,
    "access found a missing file",
  );

  await expectFileEmpty(commandFile("copyFile", {
    from: `${workRoot}/nested/input.txt`,
    to: `${workRoot}/nested/copy.txt`,
  }));
  await expectFileEmpty(commandFile("rename", {
    from: `${workRoot}/nested/copy.txt`,
    to: `${workRoot}/nested/renamed.txt`,
  }));
  const renamed = await invokeOk(
    commandFile("readFile", {
      path: `${workRoot}/nested/renamed.txt`,
      encoding: "utf8",
    }),
    "file",
  );
  assert(
    renamed.payload.payload.contents === "cefari smoke text\n",
    "renamed contents changed",
  );

  await expectError(
    commandFile("readFile", {
      path: "../outside.txt",
      encoding: "utf8",
    }),
    "invalidCommand",
  );
}

async function workerSmoke() {
  const appData = await invokeOk(commandFile("appDataDir"), "file");
  const appDataPath = appData.payload.payload.displayPath;
  const inputPath = `${appDataPath}/smoke/work/nested/input.txt`;
  const outputPath = `${appDataPath}/smoke/work/worker-output.txt`;
  const deniedPath = `${appDataPath}/smoke/result.json`;

  const spawned = await invokeWorkerOk("spawn", {
    worker: "smoke-worker",
    inputJson: JSON.stringify(null),
  }, "spawned");
  assert(spawned.worker === "smoke-worker", "spawned the wrong worker");

  const startedPromise = waitForWorkerEvent(spawned.id, "worker.message", (event) =>
    parseWorkerMessage(event).phase === "started"
  );
  const deniedPromise = waitForWorkerEvent(spawned.id, "worker.message", (event) =>
    parseWorkerMessage(event).phase === "permission-denied"
  );
  const invoked = await invokeWorkerOk("invoke", {
    id: spawned.id,
    method: "transform",
    inputJson: JSON.stringify({ inputPath, outputPath, deniedPath }),
  }, "invoked");

  const started = parseWorkerMessage(await startedPromise);
  assert(started.phase === "started", "worker did not report start");

  const denied = parseWorkerMessage(await deniedPromise);
  assert(
    denied.phase === "permission-denied",
    "worker did not report denied read",
  );

  const result = JSON.parse(invoked.outputJson);
  assert(invoked.method === "transform", "worker invoke method mismatch");
  assert(result.denied === true, "worker denied read was not enforced");
  assert(
    result.uppercased === "CEFARI SMOKE TEXT\n",
    "worker result changed",
  );

  const workerOutput = await invokeOk(
    commandFile("readFile", {
      path: `${workRoot}/worker-output.txt`,
      encoding: "utf8",
    }),
    "file",
  );
  assert(
    workerOutput.payload.payload.contents === "CEFARI SMOKE TEXT\n",
    "worker output file contents changed",
  );

  const firstTerminated = await invokeWorkerOk("terminate", { id: spawned.id }, "terminated");
  assert(firstTerminated.id === spawned.id, "terminated first worker id mismatch");
  await waitForWorkerEvent(spawned.id, "worker.exited");
  const listAfterExit = await invokeWorkerOk("list", undefined, "list");
  assert(
    listAfterExit.workers.some((worker) =>
      worker.id === spawned.id && worker.status === "exited"
    ),
    "worker list did not include exited worker",
  );

  const held = await invokeWorkerOk("spawn", {
    worker: "smoke-worker",
    inputJson: JSON.stringify(null),
  }, "spawned");
  const terminated = await invokeWorkerOk("terminate", { id: held.id }, "terminated");
  assert(terminated.id === held.id, "terminated worker id mismatch");
  await waitForWorkerEvent(held.id, "worker.exited");

  return { firstWorker: spawned.id, terminatedWorker: held.id };
}

async function postReloadSmoke() {
  record("post-reload");
  await cefari.logs.info("smoke.frontend.started", {
    phase: "post-reload",
  });

  const updateState = await invokeOk({ command: "updateState" }, "updateState");
  assert(
    updateState.payload.state === "notConfigured",
    "updates should be unconfigured",
  );

  const updateCheck = await invokeOk({ command: "updateCheck" }, "updateCheck");
  assert(
    updateCheck.payload.state === "notConfigured",
    "update check should stay local",
  );
  assert(
    updateCheck.payload.version === null,
    "unconfigured update check should not report a version",
  );
  assert(
    updateCheck.payload.updateId === null,
    "unconfigured update check should not report an id",
  );
  await expectError(
    { command: "updateApply", payload: { updateId: null } },
    "unsupported",
  );

  await expectError({
    command: "openExternalUrl",
    payload: { url: "file:///tmp/cefari-smoke" },
  }, "invalidCommand");
  await expectError({
    command: "notification",
    payload: { notification: "permissionState" },
  }, "unsupported");

  const service = await invoke({ command: "serviceStatus" });
  if (service.outcome.status === "ok") {
    assert(
      service.outcome.payload.result === "serviceStatus",
      "serviceStatus result mismatch",
    );
    assert(
      typeof service.outcome.payload.payload.status === "string",
      "serviceStatus did not include a status string",
    );
  } else {
    assert(
      service.outcome.payload.code === "unsupported",
      `serviceStatus returned ${service.outcome.payload.code}`,
    );
  }

  await fileSmoke();
  await daemonRoundTripSmoke();
  const worker = await workerSmoke();

  const title = await invokeOk({
    command: "windowSetTitle",
    payload: { title: "Cefari Smoke PASS" },
  }, "window");
  assert(
    title.payload.title === "Cefari Smoke PASS",
    "window title did not update",
  );
  assert(
    title.payload.focused === false,
    "background smoke window became focused",
  );

  await writeResult("pass", {
    window: title.payload,
    worker,
    service: service.outcome.status === "ok"
      ? service.outcome.payload.payload
      : service.outcome.payload,
    daemonRoundTrip: true,
  });
  await invokeOk({ command: "appQuit" }, "empty");
}

async function main() {
  try {
    if (sessionStorage.getItem(phaseKey) !== "after-reload") {
      await preReloadSmoke();
      return;
    }

    sessionStorage.removeItem(phaseKey);
    await postReloadSmoke();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    record("fail", message);
    try {
      await writeResult("fail", { error: message });
    } catch (writeError) {
      record(
        "failed to write result",
        writeError instanceof Error ? writeError.message : String(writeError),
      );
    }
    try {
      await invokeOk({
        command: "windowSetTitle",
        payload: { title: "Cefari Smoke FAIL" },
      }, "window");
    } catch (_error) {
      // Keep the original smoke failure as the result.
    }
    try {
      await invokeOk({ command: "appQuit" }, "empty");
    } catch (_error) {
      // The watchdog will stop the process if the bridge is already unavailable.
    }
  }
}

main();
