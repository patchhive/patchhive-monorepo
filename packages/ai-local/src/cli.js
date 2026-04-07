#!/usr/bin/env node

import { startGateway, resolveGatewayConfig } from "./index.js";

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
