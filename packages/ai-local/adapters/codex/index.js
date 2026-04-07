#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import readline from "node:readline";
import { Codex } from "@openai/codex-sdk";

const DEFAULT_MODELS = [
  "gpt-5.4",
  "gpt-5.4-mini",
  "gpt-5.3-codex",
  "gpt-5.2-codex",
  "gpt-5.2",
  "gpt-5.1-codex-max",
  "gpt-5.1-codex-mini",
];
const DEFAULT_TIMEOUT_MS = 120_000;

function parseModels(value) {
  const parsed = String(value || "")
    .split(",")
    .map(item => item.trim())
    .filter(Boolean);
  return parsed.length ? parsed : DEFAULT_MODELS;
}

function codexLoginPresent() {
  const authPath = `${os.homedir()}/.codex/auth.json`;
  return fs.existsSync(authPath);
}

function toPositiveInt(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function contentPartToText(part) {
  if (typeof part === "string") return part;
  if (!part) return "";
  if (part.type === "text" || part.type === "input_text" || part.type === "output_text") {
    return part.text || "";
  }
  if (typeof part === "object") return JSON.stringify(part);
  return String(part);
}

function contentToText(content) {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) return content.map(contentPartToText).filter(Boolean).join("\n");
  if (content && typeof content === "object") return contentPartToText(content);
  return "";
}

function messagesToPrompt(messages) {
  return (Array.isArray(messages) ? messages : [])
    .map(message => {
      const role = String(message?.role || "user").toUpperCase();
      const text = contentToText(message?.content).trim();
      return text ? `${role}:\n${text}` : "";
    })
    .filter(Boolean)
    .join("\n\n");
}

function success(id, result) {
  return JSON.stringify({
    jsonrpc: "2.0",
    id,
    result,
  });
}

function failure(id, code, message, retryable = false) {
  return JSON.stringify({
    jsonrpc: "2.0",
    id,
    error: {
      code,
      message,
      retryable,
    },
  });
}

const models = parseModels(process.env.PATCHHIVE_AI_CODEX_MODELS);
const client = new Codex();

async function runCompletion(params = {}) {
  const model = String(params.model || process.env.PATCHHIVE_AI_CODEX_MODEL || models[0]);
  const timeoutMs = toPositiveInt(params.timeout_ms, DEFAULT_TIMEOUT_MS);
  const prompt = messagesToPrompt(params.messages);

  if (!prompt) {
    throw Object.assign(new Error("No prompt messages were provided to Codex."), {
      code: "invalid_request",
      retryable: false,
    });
  }

  const thread = client.startThread({
    model,
    approvalPolicy: "never",
    sandboxMode: "read-only",
    networkAccessEnabled: false,
    webSearchEnabled: false,
    workingDirectory: process.env.PATCHHIVE_AI_WORKDIR || process.cwd(),
    skipGitRepoCheck: true,
  });

  const turn = await thread.run(prompt, {
    signal: AbortSignal.timeout(timeoutMs),
  });

  return {
    provider: "codex",
    model,
    text: turn.finalResponse?.trim() || "",
    usage: turn.usage || null,
  };
}

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

for await (const line of rl) {
  if (!line.trim()) continue;

  let message;
  try {
    message = JSON.parse(line);
  } catch {
    process.stdout.write(`${failure(null, "invalid_request", "Request was not valid JSON.")}\n`);
    continue;
  }

  const { id = null, method, params = {} } = message;

  if (method === "initialize") {
    process.stdout.write(`${success(id, {
      adapter: "codex",
      protocol_version: 1,
      ready: true,
    })}\n`);
    continue;
  }

  if (method === "health") {
    process.stdout.write(`${success(id, {
      ok: true,
      adapter: "codex",
      logged_in: codexLoginPresent(),
      models,
    })}\n`);
    continue;
  }

  if (method === "list_models") {
    process.stdout.write(`${success(id, { models })}\n`);
    continue;
  }

  if (method === "complete") {
    try {
      const result = await runCompletion(params);
      process.stdout.write(`${success(id, result)}\n`);
    } catch (error) {
      process.stdout.write(
        `${failure(
          id,
          error?.code || "internal_error",
          error instanceof Error ? error.message : String(error),
          Boolean(error?.retryable),
        )}\n`,
      );
    }
    continue;
  }

  if (method === "shutdown") {
    process.stdout.write(`${success(id, { ok: true })}\n`);
    process.exit(0);
  }

  process.stdout.write(`${failure(id, "invalid_request", `Unknown method: ${String(method || params?.method || "")}`)}\n`);
}
