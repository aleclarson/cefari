import { defineConfig } from "cefari";

export default defineConfig({
  app: {
    projectName: "cefari-smoke",
    name: "Cefari Smoke",
    identifier: "dev.cefari.smoke",
  },
  frontend: {
    dist: "frontend",
    devPort: 5273,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Cefari Smoke",
    version: "0.1.0",
  },
});
