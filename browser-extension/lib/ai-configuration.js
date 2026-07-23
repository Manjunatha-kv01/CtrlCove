// @ts-check

export const MAX_VISUAL_AI_MODEL_LENGTH = 120;
export const MAX_VISUAL_AI_API_KEY_LENGTH = 1_024;
const MODEL_IDENTIFIER_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:/-]*$/;

/** @param {unknown} value */
export function normalizeVisualAiModel(value) {
  if (typeof value !== "string") throw new Error("Enter a model name before enabling visual AI.");
  const model = value.trim();
  if (!model) throw new Error("Enter a model name before enabling visual AI.");
  if (model.length > MAX_VISUAL_AI_MODEL_LENGTH || !MODEL_IDENTIFIER_PATTERN.test(model)) {
    throw new Error("Visual AI model names must be at most 120 characters and use safe identifier characters.");
  }
  return model;
}

/** @param {unknown} value */
export function normalizeVisualAiApiKey(value) {
  if (typeof value !== "string" || !value) {
    throw new Error("Enter an API key for this browser session before enabling visual AI.");
  }
  if (value.length > MAX_VISUAL_AI_API_KEY_LENGTH || /\s/.test(value)) {
    throw new Error("Visual AI API keys must be at most 1024 characters and cannot contain whitespace.");
  }
  return value;
}
