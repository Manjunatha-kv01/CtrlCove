// @ts-check

export const MAX_VISUAL_RESPONSE_BYTES = 256 * 1024;
export const MAX_VISUAL_GUIDE_CONTENT_BYTES = 16 * 1024;
const MAX_VISUAL_GUIDE_SUMMARY_LENGTH = 1_000;
const MAX_VISUAL_GUIDE_STEP_LENGTH = 1_000;
const MAX_VISUAL_GUIDE_STEPS = 5;

/** @param {Response} response @param {number} [maxBytes] */
export async function parseBoundedVisualResponse(response, maxBytes = MAX_VISUAL_RESPONSE_BYTES) {
  const contentType = response.headers.get("content-type") ?? "";
  if (!/^application\/json(?:\s*;|\s*$)/i.test(contentType)) {
    throw new Error("Visual AI response must use an application/json content type.");
  }

  const contentLength = Number(response.headers.get("content-length"));
  if (Number.isFinite(contentLength) && contentLength > maxBytes) {
    throw new Error("Visual AI response exceeds CYMOS's 256 KiB response budget.");
  }

  const body = response.body;
  if (!body) throw new Error("Visual AI response did not include a body.");
  const reader = body.getReader();
  /** @type {Uint8Array[]} */
  const chunks = [];
  let totalBytes = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      totalBytes += value.byteLength;
      if (totalBytes > maxBytes) {
        await reader.cancel();
        throw new Error("Visual AI response exceeds CYMOS's 256 KiB response budget.");
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const bytes = new Uint8Array(totalBytes);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  const text = new TextDecoder().decode(bytes);
  if (!text.trim()) throw new Error("Visual AI response was empty.");
  try {
    return JSON.parse(text);
  } catch {
    throw new Error("Visual AI response did not contain valid JSON.");
  }
}

/** @param {unknown} payload @param {number} [maxBytes] */
export function parseVisualGuideContent(payload, maxBytes = MAX_VISUAL_GUIDE_CONTENT_BYTES) {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("Visual AI returned an invalid response.");
  }

  const candidate = /** @type {{ choices?: unknown }} */ (payload);
  if (!Array.isArray(candidate.choices) || candidate.choices.length === 0) {
    throw new Error("Visual AI response did not include a guide.");
  }
  const firstChoice = /** @type {{ message?: { content?: unknown } }} */ (candidate.choices[0]);
  const content = firstChoice.message?.content;
  if (typeof content !== "string") {
    throw new Error("Visual AI response did not contain JSON content.");
  }
  if (new TextEncoder().encode(content).byteLength > maxBytes) {
    throw new Error("Visual AI guide content exceeds CYMOS's 16 KiB guide budget.");
  }

  let guide;
  try {
    guide = JSON.parse(content);
  } catch {
    throw new Error("Visual AI guide content did not contain valid JSON.");
  }
  assertVisualGuideSchema(guide);
  return guide;
}

/** @param {unknown} guide */
function assertVisualGuideSchema(guide) {
  if (!guide || typeof guide !== "object" || Array.isArray(guide)) {
    throw new Error("Visual AI guide must be a JSON object.");
  }
  const candidate = /** @type {{ summary?: unknown, confidence?: unknown, steps?: unknown }} */ (guide);
  if (typeof candidate.summary !== "string" || !candidate.summary.trim() || candidate.summary.length > MAX_VISUAL_GUIDE_SUMMARY_LENGTH) {
    throw new Error("Visual AI guide must include a bounded summary.");
  }
  if (typeof candidate.confidence !== "number" || !Number.isFinite(candidate.confidence) || candidate.confidence < 0 || candidate.confidence > 1) {
    throw new Error("Visual AI guide confidence must be between 0 and 1.");
  }
  if (!Array.isArray(candidate.steps) || candidate.steps.length === 0 || candidate.steps.length > MAX_VISUAL_GUIDE_STEPS) {
    throw new Error("Visual AI guide must include one to five steps.");
  }
  if (candidate.steps.some((step) => typeof step !== "string" || !step.trim() || step.length > MAX_VISUAL_GUIDE_STEP_LENGTH)) {
    throw new Error("Visual AI guide steps must be bounded non-empty strings.");
  }
}
