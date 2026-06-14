import { defineConfig } from "@cefari/cli";

export default defineConfig({
  app: {
    projectName: "vite-react-basic",
    name: "Vite React Basic",
    identifier: "dev.cefari.vite-react-basic",
  },
  frontend: {
    dist: "frontend/dist",
    buildCommand: ["deno", "task", "build:frontend"],
    devCommand: [
      "deno",
      "task",
      "dev:frontend",
      "--host",
      "127.0.0.1",
      "--port",
      "{port}",
    ],
    devPort: 5111,
  },
  daemon: {
    entry: "daemon/main.ts",
  },
  package: {
    productName: "Vite React Basic",
    version: "0.1.0",
  },
});
