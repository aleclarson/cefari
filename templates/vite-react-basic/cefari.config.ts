import { defineConfig } from "cefari";

export default defineConfig(({ mode }) => ({
  app: {
    projectName: "vite-react-basic",
    name: "Vite React Basic",
    identifier: "dev.cefari.vite-react-basic",
    icon: "assets/512x512.png",
  },
  vite: {
    root: "frontend",
    configFile: "frontend/vite.config.js",
    devPort: 5111,
  },
  package: {
    productName: "Vite React Basic",
    version: mode === "production" ? "0.1.0" : "0.1.0-dev",
  },
}));
