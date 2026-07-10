import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

const apiProxy = {
  "/api": {
    target: "http://127.0.0.1:8110",
    changeOrigin: true,
    rewrite: (path) => path.replace(/^\/api/, ""),
  },
};

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: apiProxy,
  },
  preview: {
    proxy: apiProxy,
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    preserveSymlinks: true,
    alias: [
      {
        find: "@patchhivehq/product-shell/auth",
        replacement: fileURLToPath(new URL("../../../packages/product-shell/src/auth.js", import.meta.url)),
      },
    ],
  },
});
