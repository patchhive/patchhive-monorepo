const NON_TEXT_MODEL_PATTERNS = [
  /(^|[-_/:.])audio($|[-_/:.])/,
  /(^|[-_/:.])asr($|[-_/:.])/,
  /(^|[-_/:.])classifier($|[-_/:.])/,
  /(^|[-_/:.])clip($|[-_/:.])/,
  /(^|[-_/:.])dall[-_ ]?e($|[-_/:.])/,
  /(^|[-_/:.])diffusion($|[-_/:.])/,
  /(^|[-_/:.])embed(ding|dings)?($|[-_/:.])/,
  /(^|[-_/:.])flux($|[-_/:.])/,
  /(^|[-_/:.])guard($|[-_/:.])/,
  /(^|[-_/:.])image($|[-_/:.])/,
  /(^|[-_/:.])imagen($|[-_/:.])/,
  /(^|[-_/:.])moderation($|[-_/:.])/,
  /(^|[-_/:.])ocr($|[-_/:.])/,
  /(^|[-_/:.])preview($|[-_/:.])/,
  /(^|[-_/:.])rerank(er)?($|[-_/:.])/,
  /(^|[-_/:.])reward($|[-_/:.])/,
  /(^|[-_/:.])sdxl($|[-_/:.])/,
  /(^|[-_/:.])siglip($|[-_/:.])/,
  /(^|[-_/:.])speech($|[-_/:.])/,
  /(^|[-_/:.])stt($|[-_/:.])/,
  /(^|[-_/:.])test($|[-_/:.])/,
  /(^|[-_/:.])tts($|[-_/:.])/,
  /(^|[-_/:.])vl($|[-_/:.])/,
  /(^|[-_/:.])video($|[-_/:.])/,
  /(^|[-_/:.])whisper($|[-_/:.])/,
  /bge[-_./:]?m3/,
  /beta[-_./:]?test/,
  /content[-_./:]?safety/,
  /cross[-_./:]?encoder/,
  /depth[-_./:]?anything/,
  /donut[-_./:]?(base|large|small)?/,
  /e5[-_./:]?(base|large|small)/,
  /experimental/,
  /florence[-_./:]?2/,
  /image[-_./:]?generation/,
  /jina[-_./:]?embedding/,
  /layout[-_./:]?lm/,
  /llava/,
  /mini[-_./:]?cpm[-_./:]?v/,
  /molmo/,
  /omni[-_./:]?moderation/,
  /pixtral/,
  /qwen[-_./:]?2(\.5)?[-_./:]?vl/,
  /qwen[-_./:]?vl/,
  /safety[-_./:]?checker/,
  /stable[-_./:]?diffusion/,
  /text[-_./:]?embedding/,
  /text[-_./:]?to[-_./:]?speech/,
  /transcrib(e|er|ing|tion)/,
  /voice[-_./:]?(activity|clone|synthesis)/,
  /voyage[-_./:]?(2|3|code|finance|law|large|lite|multilingual)/,
  /vision[-_./:]?(encoder|tower|only)/,
];

const FREE_MODEL_PATTERNS = [
  /(^|[-_/:.])free($|[-_/:.])/,
  /:free$/,
];

const AGENT_UNREADY_MODEL_PATTERNS = [
  /(^|[-_/:.])base($|[-_/:.])/,
  /(^|[-_/:.])completion($|[-_/:.])/,
  /(^|[-_/:.])router($|[-_/:.])/,
];

const MIN_AGENT_CONTEXT_TOKENS = 16_384;
const AGENT_CONTROL_PARAMETERS = new Set(["tools", "structured_outputs", "response_format"]);

function normalizeModelId(modelId) {
  return String(modelId || "").trim().toLowerCase();
}

export function isPatchHiveTextModel(modelId) {
  const normalized = normalizeModelId(modelId);
  if (!normalized) return false;
  return !NON_TEXT_MODEL_PATTERNS.some((pattern) => pattern.test(normalized));
}

export function isFreeProviderModel(modelId) {
  const normalized = normalizeModelId(modelId);
  if (!normalized) return false;
  return FREE_MODEL_PATTERNS.some((pattern) => pattern.test(normalized));
}

function metadataForModel(modelId, metadata = {}) {
  if (!metadata || typeof metadata !== "object") return null;
  return metadata[modelId] || metadata[normalizeModelId(modelId)] || null;
}

export function isAgentReadyProviderModel(modelId, metadata = null) {
  const normalized = normalizeModelId(modelId);
  if (!normalized || AGENT_UNREADY_MODEL_PATTERNS.some((pattern) => pattern.test(normalized))) {
    return false;
  }
  if (!metadata || typeof metadata !== "object") return true;

  const architecture = metadata.architecture || {};
  const inputModalities = Array.isArray(architecture.input_modalities)
    ? architecture.input_modalities.map(normalizeModelId)
    : [];
  const outputModalities = Array.isArray(architecture.output_modalities)
    ? architecture.output_modalities.map(normalizeModelId)
    : [];
  if (inputModalities.length && !inputModalities.includes("text")) return false;
  if (outputModalities.length && !outputModalities.includes("text")) return false;

  const contextLength = Number(metadata.context_length);
  if (Number.isFinite(contextLength) && contextLength > 0 && contextLength < MIN_AGENT_CONTEXT_TOKENS) {
    return false;
  }

  if (metadata.expiration_date) {
    const expiration = Date.parse(metadata.expiration_date);
    if (Number.isFinite(expiration) && expiration <= Date.now()) return false;
  }

  const supported = Array.isArray(metadata.supported_parameters)
    ? metadata.supported_parameters.map(normalizeModelId)
    : [];
  if (supported.length) {
    const controlsOutput = supported.some((parameter) => AGENT_CONTROL_PARAMETERS.has(parameter));
    const limitsOutput = supported.includes("max_tokens") || supported.includes("max_completion_tokens");
    if (!controlsOutput || !limitsOutput) return false;
  }

  return true;
}

export function filterPatchHiveTextModels(modelIds = [], options = {}) {
  const freeOnly = Boolean(options.freeOnly);
  const agentReadyOnly = Boolean(options.agentReadyOnly);
  const metadata = options.metadata || {};
  const kept = [];
  const dropped = [];
  const agentHidden = [];
  const freeHidden = [];
  modelIds.forEach((modelId) => {
    if (!isPatchHiveTextModel(modelId)) {
      dropped.push(modelId);
      return;
    }
    if (agentReadyOnly && !isAgentReadyProviderModel(modelId, metadataForModel(modelId, metadata))) {
      agentHidden.push(modelId);
      return;
    }
    if (freeOnly && !isFreeProviderModel(modelId)) {
      freeHidden.push(modelId);
      return;
    }
    kept.push(modelId);
  });
  return { agentHidden, dropped, freeHidden, models: kept };
}
