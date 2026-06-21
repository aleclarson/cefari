import { cefari } from "cefari/app";
import type { FeatureModule } from "./types.ts";

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const features: FeatureModule[] = [
  {
    id: "bridge-availability",
    name: "Bridge availability",
    category: "Bridge",
    description: "Confirms the web UI is attached to a Cefari host and records the current event stream briefly.",
    async run({ log }) {
      const events: unknown[] = [];
      const off = cefari.onAnyEvent((event) => events.push(event));
      log("listening for host events for 750ms");
      await wait(750);
      off();
      return { available: cefari.isAvailable(), observedEvents: events };
    },
  },
  {
    id: "app-info",
    name: "App info",
    category: "Desktop",
    description: "Reads application metadata exposed by the desktop shell.",
    async run() {
      return { currentWindow: await cefari.desktop.window.current(), windows: await cefari.desktop.window.list() };
    },
  },
  {
    id: "window-state",
    name: "Window state round trip",
    category: "Desktop",
    description: "Queries window state, briefly changes the title, then restores it.",
    async run({ log }) {
      const before = await cefari.desktop.window.current();
      log("captured current window state", before);
      await cefari.desktop.window.setTitle("Cefari Playground — running window test");
      await wait(500);
      await cefari.desktop.window.setTitle("Cefari Playground");
      return { before, titleRestored: true };
    },
  },
  {
    id: "notification",
    name: "Notification",
    category: "System",
    description: "Requests notification permission and sends a playground notification.",
    async run() {
      const permission = await cefari.notifications.requestPermission();
      const capabilities = await cefari.notifications.capabilities();
      const sent = await cefari.notifications.send({ title: "Cefari Playground", body: "Notification feature completed." });
      return { permission, capabilities, sent };
    },
  },
  {
    id: "dialog",
    name: "Dialog prompt",
    category: "System",
    description: "Opens a host dialog so modal UI behavior can be checked manually.",
    async run() {
      return await cefari.dialogs.saveFile({ title: "Choose a place for a playground test file", defaultName: "cefari-playground.txt" });
    },
  },
  {
    id: "filesystem",
    name: "Filesystem scratch file",
    category: "Files",
    description: "Writes, reads, stats, lists, and removes a temporary playground file.",
    async run({ log }) {
      const path = `cefari-playground-${Date.now()}.txt`;
      await cefari.fs.writeFile(path, "hello from the Cefari playground\n");
      log("wrote scratch file", path);
      const text = await cefari.fs.readFile(path, "utf8");
      const stats = await cefari.fs.stat(path);
      const entries = await cefari.fs.readdir(".");
      await cefari.fs.rm(path);
      return { path, text, stats, entries: entries.slice(0, 10) };
    },
  },
  {
    id: "worker-compute",
    name: "Worker compute",
    category: "Workers",
    description: "Runs CPU and message traffic through a configured Cefari worker.",
    async run({ log }) {
      const handle = await cefari.workers.spawn("limits" as never, { label: "playground" } as never);
      const off = handle.onMessage((message) => log("worker message", message));
      try {
        const output = await handle.invoke("compute" as never, { iterations: 12000 } as never);
        const workers = await cefari.workers.list();
        return { output, workers };
      } finally {
        off();
        await handle.terminate();
      }
    },
  },
  {
    id: "logs",
    name: "Logs",
    category: "Diagnostics",
    description: "Emits a structured log entry through the Cefari logging API.",
    async run() {
      await cefari.logs.info("playground feature log", { feature: "logs", at: new Date().toISOString() });
      return { written: true };
    },
  },
];
