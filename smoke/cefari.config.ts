import { defineConfig } from "cefari";

export default defineConfig({
  app: {
    projectName: "cefari-smoke",
    name: "Cefari Smoke",
    identifier: "dev.cefari.smoke",
  },
  vite: {
    root: "frontend",
    configFile: false,
    devPort: 5273,
  },
  workers: {
    "smoke-worker": {
      entry: "workers/smoke-worker.ts",
      permissions: {
        read: ["$appData/smoke/work"],
        write: ["$appData/smoke/work"],
      },
    },
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Cefari Smoke",
    version: "0.1.0",
  },
});
