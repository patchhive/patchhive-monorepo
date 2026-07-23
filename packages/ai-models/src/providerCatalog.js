export const AI_PROVIDERS = {
  anthropic: {
    label: "Anthropic",
    icon: "◈",
    color: "#d97757",
    keyHint: "sk-ant-...",
    liveHint: "Uses Anthropic /v1/models when a key is supplied or saved globally.",
  },
  openai: {
    label: "OpenAI / Compatible",
    icon: "⬡",
    color: "#2aa876",
    keyHint: "sk-...",
    liveHint: "Uses PatchHive Local AI when configured, otherwise OpenAI-compatible /models.",
  },
  gemini: {
    label: "Gemini",
    icon: "✦",
    color: "#5a8dee",
    keyHint: "AIza...",
    liveHint: "Uses Google Generative Language model discovery when a key is supplied.",
  },
  groq: {
    label: "Groq",
    icon: "▰",
    color: "#f97316",
    keyHint: "gsk_...",
    liveHint: "Uses Groq's OpenAI-compatible /models endpoint.",
  },
  custom: {
    label: "Custom OpenAI-Compatible",
    icon: "◇",
    color: "#3a9fb3",
    keyHint: "provider key",
    liveHint: "Uses a custom OpenAI-compatible base URL such as http://localhost:8787/v1.",
  },
  ollama: {
    label: "Ollama",
    icon: "●",
    color: "#8b8bff",
    noKey: true,
    keyHint: "",
    liveHint: "Uses local Ollama /api/tags from the configured base URL.",
  },
};

export const DEFAULT_PROVIDER_MODELS = {
  anthropic: [
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
    "claude-sonnet-4-20250514",
  ],
  openai: [
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.3-codex",
    "gpt-5.2-codex",
    "gpt-5.1",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5.1-codex",
    "gpt-5.1-codex-mini",
    "gpt-5.1-codex-max",
    "gpt-5-codex",
    "gpt-5",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "o3",
    "o4-mini",
    "o3-mini",
  ],
  gemini: [
    "gemini-2.5-pro",
    "gemini-2.0-flash",
    "gemini-2.0-flash-lite",
    "gemini-1.5-pro",
  ],
  groq: [
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
  ],
  custom: [
    "gpt-4.1-mini",
    "qwen2.5-coder",
    "llama3.2",
  ],
  ollama: [
    "llama3.2",
    "codellama",
    "deepseek-coder",
    "qwen2.5-coder",
  ],
};

export function providerOptions(providers = AI_PROVIDERS) {
  return Object.entries(providers).map(([value, meta]) => ({
    v: value,
    l: `${meta.icon} ${meta.label}`,
  }));
}

export function defaultModelForProvider(provider, fallbackModels = DEFAULT_PROVIDER_MODELS) {
  return fallbackModels?.[provider]?.[0] || "";
}

export function modelListForProvider(provider, liveModels = {}, fallbackModels = DEFAULT_PROVIDER_MODELS) {
  return liveModels?.[provider] || fallbackModels?.[provider] || [];
}
