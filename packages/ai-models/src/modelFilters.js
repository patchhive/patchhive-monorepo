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

export function filterPatchHiveTextModels(modelIds = [], options = {}) {
  const freeOnly = Boolean(options.freeOnly);
  const kept = [];
  const dropped = [];
  const freeHidden = [];
  modelIds.forEach((modelId) => {
    if (!isPatchHiveTextModel(modelId)) {
      dropped.push(modelId);
      return;
    }
    if (freeOnly && !isFreeProviderModel(modelId)) {
      freeHidden.push(modelId);
      return;
    }
    kept.push(modelId);
  });
  return { dropped, freeHidden, models: kept };
}
