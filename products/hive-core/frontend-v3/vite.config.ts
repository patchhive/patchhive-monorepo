import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import tsconfigPaths from "vite-tsconfig-paths";
import { tanstackRouter } from "@tanstack/router-plugin/vite";

// SPA, not TanStack Start. HiveCore is a single-operator console behind an API key:
// SSR buys nothing here, and a Node server would be a second place that holds the
// operator credential. Every fact comes from the Rust backend on VITE_API_URL.
export default defineConfig({
  plugins: [
    tsconfigPaths(),
    tanstackRouter({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwindcss(),
  ],
  server: { port: 5311 },
  preview: { port: 4311 },
});
