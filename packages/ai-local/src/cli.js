#!/usr/bin/env node

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { loadEnvFile } from "node:process";
import { fileURLToPath } from "node:url";

const explicitEnv = String(process.env.PATCHHIVE_ENV_FILE || "").trim();
const canonicalEnv = resolve(dirname(fileURLToPath(import.meta.url)), "../../../.env");
if (explicitEnv) {
  loadEnvFile(explicitEnv);
} else if (existsSync(canonicalEnv)) {
  loadEnvFile(canonicalEnv);
}

// Load adapters only after canonical environment values are available; some
// provider modules establish their bounded fallback catalogs at module load.
const { startGateway, resolveGatewayConfig } = await import("./index.js");

const config = resolveGatewayConfig();
const gateway = await startGateway(config);

console.log(
  `[patchhive-ai-local] listening on http://${config.host}:${config.port} ` +
  `using ${config.providerOrder.join(" -> ")}`,
);

const shutdown = async signal => {
  console.log(`[patchhive-ai-local] shutting down on ${signal}`);
  await gateway.close();
  process.exit(0);
};

process.on("SIGINT", () => void shutdown("SIGINT"));
process.on("SIGTERM", () => void shutdown("SIGTERM"));
