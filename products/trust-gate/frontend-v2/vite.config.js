import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8020",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: [
      {
        find: "@patchhivehq/ui-v2/styles.css",
        replacement: fileURLToPath(new URL("../../../packages/ui-v2/src/styles.css", import.meta.url)),
      },
      {
        find: "@patchhivehq/ui-v2",
        replacement: fileURLToPath(new URL("../../../packages/ui-v2/src/index.js", import.meta.url)),
      },
      {
        find: "@patchhivehq/product-shell/auth",
        replacement: fileURLToPath(new URL("../../../packages/product-shell/src/auth.js", import.meta.url)),
      },
    ],
  },
});
