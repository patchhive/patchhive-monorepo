import http from "node:http";
import { randomUUID } from "node:crypto";

import { CopilotClient } from "@github/copilot-sdk";
import { Codex } from "@openai/codex-sdk";

const DEFAULT_TIMEOUT_MS = 120_000;
const DEFAULT_PROVIDER_ORDER = ["codex", "copilot"];
const DEFAULT_MODELS = {
  codex: "gpt-5.4",
  copilot: "gpt-5",
};

function parseCsv(value, fallback = []) {
  return (value || "")
    .split(",")
    .map(item => item.trim().toLowerCase())
    .filter(Boolean)
    .filter((item, index, list) => list.indexOf(item) === index)
    .filter(item => fallback.length === 0 || fallback.includes(item));
}

function parseBool(value, fallback) {
  if (value == null || value === "") return fallback;
  const normalized = String(value).trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) return true;
  if (["0", "false", "no", "off"].includes(normalized)) return false;
  return fallback;
}

function toPositiveInt(value, fallback) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function normalizeProvider(value) {
  const normalized = String(value || "").trim().toLowerCase();
  return DEFAULT_PROVIDER_ORDER.includes(normalized) ? normalized : null;
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

function responseInputToMessages(input) {
  if (typeof input === "string") return [{ role: "user", content: input }];
  if (!Array.isArray(input)) return [];

  return input.flatMap(item => {
    if (typeof item === "string") return [{ role: "user", content: item }];
    if (!item || typeof item !== "object") return [];
    if (item.type === "message") {
      return [{ role: item.role || "user", content: contentToText(item.content) }];
    }
    return [{ role: "user", content: contentToText(item.content ?? item) }];
  });
}

function usageToOpenAi(usage) {
  if (!usage) return undefined;
  const promptTokens = usage.input_tokens + (usage.cached_input_tokens || 0);
  const completionTokens = usage.output_tokens || 0;
  return {
    prompt_tokens: promptTokens,
    completion_tokens: completionTokens,
    total_tokens: promptTokens + completionTokens,
  };
}

function pickModel(config, provider, requestedModel) {
  return requestedModel || config.defaultModels[provider] || DEFAULT_MODELS[provider];
}

function extractRequestProvider(req, body, config) {
  const explicit = normalizeProvider(
    body?.patchhive_provider || body?.provider || req.headers["x-patchhive-provider"],
  );
  if (explicit) return [explicit];
  return config.providerOrder;
}

function makeOpenAiResponse(result, attempts) {
  const id = `chatcmpl_${randomUUID()}`;
  return {
    id,
    object: "chat.completion",
    created: Math.floor(Date.now() / 1000),
    model: result.model,
    choices: [
      {
        index: 0,
        message: {
          role: "assistant",
          content: result.text,
        },
        finish_reason: "stop",
      },
    ],
    usage: usageToOpenAi(result.usage),
    patchhive: {
      provider: result.provider,
      attempts,
    },
  };
}

function makeResponsesApiResponse(result, attempts) {
  const usage = usageToOpenAi(result.usage);
  return {
    id: `resp_${randomUUID()}`,
    object: "response",
    created_at: Math.floor(Date.now() / 1000),
    model: result.model,
    output: [
      {
        id: `msg_${randomUUID()}`,
        type: "message",
        role: "assistant",
        content: [
          {
            type: "output_text",
            text: result.text,
            annotations: [],
          },
        ],
      },
    ],
    output_text: result.text,
    usage: usage
      ? {
          input_tokens: usage.prompt_tokens,
          output_tokens: usage.completion_tokens,
          total_tokens: usage.total_tokens,
        }
      : undefined,
    patchhive: {
      provider: result.provider,
      attempts,
    },
  };
}

function createErrorPayload(error, attempts = []) {
  return {
    error: {
      message: error instanceof Error ? error.message : String(error),
      type: "patchhive_gateway_error",
    },
    patchhive: { attempts },
  };
}

function setCorsHeaders(res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Patchhive-Provider");
}

function writeJson(res, status, payload, extraHeaders = {}) {
  setCorsHeaders(res);
  Object.entries(extraHeaders).forEach(([key, value]) => res.setHeader(key, value));
  res.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(payload, null, 2));
}

async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString("utf8").trim();
  return raw ? JSON.parse(raw) : {};
}

class CodexAdapter {
  constructor(config) {
    this.config = config;
    this.client = new Codex();
  }

  async complete({ model, messages, timeoutMs }) {
    const prompt = messagesToPrompt(messages);
    if (!prompt) throw new Error("No prompt messages were provided to Codex.");

    const thread = this.client.startThread({
      model,
      approvalPolicy: "never",
      sandboxMode: "read-only",
      networkAccessEnabled: false,
      webSearchEnabled: false,
      workingDirectory: this.config.workingDirectory,
      skipGitRepoCheck: true,
    });

    const turn = await thread.run(prompt, {
      signal: AbortSignal.timeout(timeoutMs),
    });

    return {
      provider: "codex",
      model,
      text: turn.finalResponse?.trim() || "",
      usage: turn.usage,
    };
  }
}

class CopilotAdapter {
  constructor(config) {
    this.config = config;
    this.client = null;
    this.startPromise = null;
  }

  async getClient() {
    if (!this.startPromise) {
      this.startPromise = (async () => {
        const client = new CopilotClient({ useLoggedInUser: true });
        await client.start();
        this.client = client;
        return client;
      })();
    }
    return this.startPromise;
  }

  async complete({ model, messages, timeoutMs }) {
    const prompt = messagesToPrompt(messages);
    if (!prompt) throw new Error("No prompt messages were provided to Copilot.");

    const client = await this.getClient();
    const session = await client.createSession({
      model,
      onPermissionRequest: () => ({ kind: "denied-by-rules" }),
    });

    try {
      const response = await session.sendAndWait({ prompt }, timeoutMs);
      return {
        provider: "copilot",
        model,
        text: response?.data?.content?.trim() || "",
        usage: null,
      };
    } finally {
      await session.disconnect();
    }
  }

  async close() {
    if (this.client) {
      await this.client.stop();
    }
  }
}

export function resolveGatewayConfig(env = process.env) {
  const providerOrder = parseCsv(
    env.PATCHHIVE_AI_PROVIDER_ORDER,
    DEFAULT_PROVIDER_ORDER,
  );

  return {
    host: env.PATCHHIVE_AI_HOST || "127.0.0.1",
    port: toPositiveInt(env.PATCHHIVE_AI_PORT, 8787),
    providerOrder: providerOrder.length ? providerOrder : [...DEFAULT_PROVIDER_ORDER],
    requestTimeoutMs: toPositiveInt(env.PATCHHIVE_AI_TIMEOUT_MS, DEFAULT_TIMEOUT_MS),
    codex: {
      workingDirectory: env.PATCHHIVE_AI_WORKDIR || process.cwd(),
    },
    copilot: {
      enabled: parseBool(env.PATCHHIVE_AI_ENABLE_COPILOT, true),
    },
    defaultModels: {
      codex: env.PATCHHIVE_AI_CODEX_MODEL || DEFAULT_MODELS.codex,
      copilot: env.PATCHHIVE_AI_COPILOT_MODEL || DEFAULT_MODELS.copilot,
    },
  };
}

export function createGateway(config = resolveGatewayConfig()) {
  const adapters = new Map();
  adapters.set("codex", new CodexAdapter(config.codex));
  if (config.copilot.enabled) {
    adapters.set("copilot", new CopilotAdapter(config.copilot));
  }

  async function completeWithFallback(req, body) {
    const attempts = [];
    const requestedProviders = extractRequestProvider(req, body, config);
    const messages = body.messages || responseInputToMessages(body.input);
    const timeoutMs = toPositiveInt(body.patchhive_timeout_ms, config.requestTimeoutMs);

    for (const provider of requestedProviders) {
      const adapter = adapters.get(provider);
      if (!adapter) {
        attempts.push({ provider, ok: false, error: "provider not enabled" });
        continue;
      }

      const model = pickModel(config, provider, body.model);
      try {
        const result = await adapter.complete({ model, messages, timeoutMs });
        attempts.push({ provider, ok: true, model });
        return { result, attempts };
      } catch (error) {
        attempts.push({
          provider,
          ok: false,
          model,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    const message = attempts.length
      ? attempts.map(attempt => `${attempt.provider}: ${attempt.error}`).join("; ")
      : "No providers were configured.";

    throw Object.assign(new Error(`All local AI providers failed. ${message}`), { attempts });
  }

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url || "/", `http://${req.headers.host || "127.0.0.1"}`);

    if (req.method === "OPTIONS") {
      setCorsHeaders(res);
      res.writeHead(204);
      res.end();
      return;
    }

    if (req.method === "GET" && url.pathname === "/health") {
      writeJson(res, 200, {
        ok: true,
        provider_order: config.providerOrder,
        providers: Array.from(adapters.keys()),
        base_url_hint: `http://${config.host}:${config.port}/v1`,
      });
      return;
    }

    if (req.method === "GET" && url.pathname === "/v1/models") {
      const data = Array.from(adapters.keys()).map(provider => ({
        id: config.defaultModels[provider] || DEFAULT_MODELS[provider],
        object: "model",
        owned_by: `patchhive-${provider}`,
      }));
      writeJson(res, 200, { object: "list", data });
      return;
    }

    if (req.method === "POST" && (url.pathname === "/v1/chat/completions" || url.pathname === "/v1/responses")) {
      let body;
      try {
        body = await readJson(req);
      } catch (error) {
        writeJson(res, 400, createErrorPayload(new Error("Invalid JSON request body.")));
        return;
      }

      if (body.stream) {
        writeJson(res, 400, createErrorPayload(new Error("Streaming is not supported yet by @patchhive/ai-local.")));
        return;
      }

      try {
        const { result, attempts } = await completeWithFallback(req, body);
        const payload = url.pathname === "/v1/responses"
          ? makeResponsesApiResponse(result, attempts)
          : makeOpenAiResponse(result, attempts);

        writeJson(res, 200, payload, {
          "X-PatchHive-Provider": result.provider,
          "X-PatchHive-Model": result.model,
        });
      } catch (error) {
        writeJson(
          res,
          502,
          createErrorPayload(error, error?.attempts || []),
        );
      }
      return;
    }

    writeJson(res, 404, createErrorPayload(new Error(`Unknown route: ${req.method} ${url.pathname}`)));
  });

  return {
    config,
    server,
    async listen() {
      await new Promise((resolve, reject) => {
        server.once("error", reject);
        server.listen(config.port, config.host, () => {
          server.off("error", reject);
          resolve();
        });
      });
      return this;
    },
    async close() {
      await Promise.all(
        Array.from(adapters.values())
          .map(adapter => adapter.close?.())
          .filter(Boolean),
      );
      await new Promise(resolve => server.close(resolve));
    },
  };
}

export async function startGateway(config = resolveGatewayConfig()) {
  const gateway = createGateway(config);
  await gateway.listen();
  return gateway;
}
