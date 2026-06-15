import { defineConfig } from "cefari";

export default defineConfig(({ mode }) => ({
  app: {
    projectName: "vite-react-basic",
    name: "Vite React Basic",
    identifier: "dev.cefari.vite-react-basic",
  },
  vite: {
    root: "frontend",
    configFile: "frontend/vite.config.js",
    devPort: 5111,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Vite React Basic",
    version: mode === "production" ? "0.1.0" : "0.1.0-dev",
  },
}));
