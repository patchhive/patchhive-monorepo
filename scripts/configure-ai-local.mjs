#!/usr/bin/env node

import { randomBytes } from "node:crypto";
import { chmod, open, readFile, rename, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const envPath = join(repoRoot, ".env");
const examplePath = join(repoRoot, ".env.example");
const tempPath = join(repoRoot, `.env.ai-local-${process.pid}.tmp`);

let source;
try {
  source = await readFile(envPath, "utf8");
} catch (error) {
  if (error?.code !== "ENOENT") throw error;
  source = await readFile(examplePath, "utf8");
}

const existingKey = readValue(source, "PATCHHIVE_AI_GATEWAY_API_KEY");
const gatewayKey = usableSecret(existingKey)
  ? existingKey
  : randomBytes(32).toString("hex");

let updated = upsert(source, "PATCHHIVE_AI_URL", "http://127.0.0.1:8787/v1");
updated = upsert(updated, "PATCHHIVE_AI_GATEWAY_API_KEY", gatewayKey);
updated = upsert(updated, "PATCHHIVE_AI_AUTOSTART", "true");

let handle;
try {
  handle = await open(tempPath, "wx", 0o600);
  await handle.writeFile(updated, "utf8");
  await handle.sync();
  await handle.close();
  handle = undefined;
  await rename(tempPath, envPath);
  await chmod(envPath, 0o600);
} finally {
  if (handle) await handle.close().catch(() => {});
  await rm(tempPath, { force: true }).catch(() => {});
}

console.log("Configured PatchHive AI gateway URL, autostart, and scoped credential in .env (credential redacted).");

function readValue(text, key) {
  for (const line of text.split(/\r?\n/)) {
    if (line.trimStart().startsWith("#")) continue;
    const separator = line.indexOf("=");
    if (separator < 0 || line.slice(0, separator).trim() !== key) continue;
    return line.slice(separator + 1).trim().replace(/^(['"])(.*)\1$/, "$2");
  }
  return "";
}

function usableSecret(value) {
  const normalized = String(value || "").trim().toLowerCase();
  return normalized.length >= 32
    && !normalized.includes("xxxxxxxx")
    && !normalized.includes("replace-me")
    && !normalized.includes("your-");
}

function upsert(text, key, value) {
  const lines = text.split(/\r?\n/).filter((line) => {
    if (line.trimStart().startsWith("#")) return true;
    const separator = line.indexOf("=");
    return separator < 0 || line.slice(0, separator).trim() !== key;
  });
  while (lines.length && !lines.at(-1).trim()) lines.pop();
  lines.push(`${key}=${value}`, "");
  return lines.join("\n");
}
