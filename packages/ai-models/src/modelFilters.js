const NON_TEXT_MODEL_PATTERNS = [
  /(^|[-_/:.])audio($|[-_/:.])/,
  /(^|[-_/:.])asr($|[-_/:.])/,
  /(^|[-_/:.])clip($|[-_/:.])/,
  /(^|[-_/:.])dall[-_ ]?e($|[-_/:.])/,
  /(^|[-_/:.])diffusion($|[-_/:.])/,
  /(^|[-_/:.])embed(ding|dings)?($|[-_/:.])/,
  /(^|[-_/:.])flux($|[-_/:.])/,
  /(^|[-_/:.])guard($|[-_/:.])/,
  /(^|[-_/:.])image($|[-_/:.])/,
  /(^|[-_/:.])imagen($|[-_/:.])/,
  /(^|[-_/:.])moderation($|[-_/:.])/,
  /(^|[-_/:.])rerank(er)?($|[-_/:.])/,
  /(^|[-_/:.])sdxl($|[-_/:.])/,
  /(^|[-_/:.])siglip($|[-_/:.])/,
  /(^|[-_/:.])speech($|[-_/:.])/,
  /(^|[-_/:.])stt($|[-_/:.])/,
  /(^|[-_/:.])tts($|[-_/:.])/,
  /(^|[-_/:.])video($|[-_/:.])/,
  /(^|[-_/:.])whisper($|[-_/:.])/,
  /bge[-_./:]?m3/,
  /content[-_./:]?safety/,
  /cross[-_./:]?encoder/,
  /e5[-_./:]?(base|large|small)/,
  /image[-_./:]?generation/,
  /jina[-_./:]?embedding/,
  /omni[-_./:]?moderation/,
  /stable[-_./:]?diffusion/,
  /text[-_./:]?embedding/,
  /text[-_./:]?to[-_./:]?speech/,
  /transcrib(e|er|ing|tion)/,
  /voice[-_./:]?(activity|clone|synthesis)/,
  /voyage[-_./:]?(2|3|code|finance|law|large|lite|multilingual)/,
];

function normalizeModelId(modelId) {
  return String(modelId || "").trim().toLowerCase();
}

export function isPatchHiveTextModel(modelId) {
  const normalized = normalizeModelId(modelId);
  if (!normalized) return false;
  return !NON_TEXT_MODEL_PATTERNS.some((pattern) => pattern.test(normalized));
}

export function filterPatchHiveTextModels(modelIds = []) {
  const kept = [];
  const dropped = [];
  modelIds.forEach((modelId) => {
    if (isPatchHiveTextModel(modelId)) {
      kept.push(modelId);
    } else {
      dropped.push(modelId);
    }
  });
  return { dropped, models: kept };
}
