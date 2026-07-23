// @ts-check

export const VISUAL_CAPTURE_BUDGET_WINDOW_MS = 60 * 60 * 1_000;
export const MAX_VISUAL_CAPTURES_PER_ORIGIN = 24;
export const MAX_TRACKED_VISUAL_CAPTURE_ORIGINS = 100;

/** @typedef {Record<string, number[]>} VisualCaptureBudgets */
/** @typedef {{ allowed: boolean, timestamps: number[] }} VisualCaptureBudgetDecision */

/** @param {unknown} value @param {number} [now] @returns {VisualCaptureBudgets} */
export function normalizeVisualCaptureBudgets(value, now = Date.now()) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  /** @type {Array<[string, number[]]>} */
  const candidates = [];
  for (const [origin, timestamps] of Object.entries(value)) {
    if (!isHttpOrigin(origin)) continue;
    const decision = visualCaptureBudgetDecision(timestamps, now);
    if (decision.timestamps.length > 0) candidates.push([origin, decision.timestamps]);
  }
  /** @type {VisualCaptureBudgets} */
  const normalized = {};
  for (const [origin, timestamps] of candidates
    .sort(([leftOrigin, leftTimestamps], [rightOrigin, rightTimestamps]) => {
      const newestDifference = (rightTimestamps[0] ?? 0) - (leftTimestamps[0] ?? 0);
      return newestDifference || leftOrigin.localeCompare(rightOrigin);
    })
    .slice(0, MAX_TRACKED_VISUAL_CAPTURE_ORIGINS)) {
    normalized[origin] = timestamps;
  }
  return normalized;
}

/** @param {unknown} previous @param {number} [now] @returns {VisualCaptureBudgetDecision} */
export function visualCaptureBudgetDecision(previous, now = Date.now()) {
  const timestamps = Array.isArray(previous)
    ? previous.filter((timestamp) => typeof timestamp === "number" && Number.isFinite(timestamp))
    : [];
  const recent = timestamps
    .filter((timestamp) => now - timestamp < VISUAL_CAPTURE_BUDGET_WINDOW_MS)
    .sort((left, right) => right - left)
    .slice(0, MAX_VISUAL_CAPTURES_PER_ORIGIN);
  return { allowed: recent.length < MAX_VISUAL_CAPTURES_PER_ORIGIN, timestamps: recent };
}

/** @param {VisualCaptureBudgetDecision} decision @param {number} [now] */
export function reserveVisualCaptureBudget(decision, now = Date.now()) {
  return [...decision.timestamps, now];
}

/** @param {VisualCaptureBudgetDecision} decision */
export function visualCaptureBudgetRemaining(decision) {
  return Math.max(0, MAX_VISUAL_CAPTURES_PER_ORIGIN - decision.timestamps.length);
}

/** @param {VisualCaptureBudgetDecision} decision */
export function visualCaptureBudgetNextAvailableAt(decision) {
  if (decision.allowed || decision.timestamps.length === 0) return null;
  return Math.min(...decision.timestamps) + VISUAL_CAPTURE_BUDGET_WINDOW_MS;
}

/** @param {string} value */
function isHttpOrigin(value) {
  try {
    const url = new URL(value);
    return (url.protocol === "https:" || url.protocol === "http:") && url.origin === value;
  } catch {
    return false;
  }
}
