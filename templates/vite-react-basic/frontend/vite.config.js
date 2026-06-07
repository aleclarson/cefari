import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@cefari/app": new URL(
        "../../../packages/cefari-app/src/mod.ts",
        import.meta.url,
      ).pathname,
    },
  },
});
