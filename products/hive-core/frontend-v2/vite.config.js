import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [
      {
        find: "@patchhivehq/ui-v2/styles.css",
        replacement: fileURLToPath(new URL("../../../packages/ui-v2/src/styles.css", import.meta.url)),
      },
      {
        find: "@patchhivehq/ui-v2",
        replacement: fileURLToPath(new URL("../../../packages/ui-v2/src/index.js", import.meta.url)),
      },
    ],
  },
});
