import { defineConfig } from "cefari";

export default defineConfig(({ mode }) => ({
  app: {
    projectName: "cefari-playground",
    name: "Cefari Playground",
    identifier: "dev.cefari.playground",
    icon: "assets/512x512.png",
  },
  vite: {
    root: "frontend",
    configFile: "frontend/vite.config.ts",
    devPort: 5123,
  },
  workers: {
    limits: {
      entry: "workers/limits.ts",
      permissions: { read: "none", write: "none", net: "none", env: "none", run: "none", ffi: "none" },
    },
  },
  package: {
    productName: "Cefari Playground",
    version: mode === "production" ? "0.1.0" : "0.1.0-dev",
  },
}));
