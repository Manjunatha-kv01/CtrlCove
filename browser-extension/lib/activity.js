// @ts-check

export const MAX_ACTIVITY_EVENTS = 20;

/**
 * @typedef {"not_requested" | "completed" | "fallback" | "unavailable" | "cooldown" | "quota"} VisualAiOutcome
 * @typedef {{ id: string, tabId: number, pageUrl: string, focus: string, source: string, visualAiAttempted: boolean, visualAiOutcome: VisualAiOutcome, capturedAt: string }} CaptureActivity
 */

/**
 * @param {{ tabId: number, guide: import("./guide-engine.js").BrowserGuide, visualAiAttempted: boolean, visualAiOutcome: VisualAiOutcome }} input
 * @returns {CaptureActivity}
 */
export function captureActivity(input) {
  const focus = input.guide.context.element.label
    || input.guide.context.element.text
    || input.guide.context.element.tagName
    || "Page context";
  return {
    id: `${input.tabId}-${input.guide.context.capturedAt}`,
    tabId: input.tabId,
    pageUrl: input.guide.context.pageUrl,
    focus: focus.slice(0, 180),
    source: input.guide.source,
    visualAiAttempted: input.visualAiAttempted,
    visualAiOutcome: input.visualAiOutcome,
    capturedAt: input.guide.context.capturedAt
  };
}

/** @param {unknown} records */
export function normalizedActivity(records) {
  if (!Array.isArray(records)) return [];
  return records
    .map((record) => normalizeRecord(record))
    .filter((record) => record !== null);
}

/** @param {unknown} record */
function normalizeRecord(record) {
  if (!record || typeof record !== "object") return null;
  const candidate = /** @type {{ id?: unknown, tabId?: unknown, pageUrl?: unknown, focus?: unknown, source?: unknown, visualAiAttempted?: unknown, visualAiOutcome?: unknown, capturedAt?: unknown }} */ (record);
  if (
    typeof candidate.id !== "string"
    || typeof candidate.tabId !== "number"
    || typeof candidate.pageUrl !== "string"
    || typeof candidate.focus !== "string"
    || typeof candidate.source !== "string"
    || typeof candidate.capturedAt !== "string"
  ) return null;
  return {
    id: candidate.id,
    tabId: candidate.tabId,
    pageUrl: candidate.pageUrl,
    focus: candidate.focus,
    source: candidate.source,
    visualAiAttempted: candidate.visualAiAttempted === true,
    visualAiOutcome: normalizeVisualAiOutcome(
      candidate.visualAiOutcome,
      candidate.source,
      candidate.visualAiAttempted === true
    ),
    capturedAt: candidate.capturedAt
  };
}

/** @param {unknown} value @param {string} source @param {boolean} visualAiAttempted */
function normalizeVisualAiOutcome(value, source, visualAiAttempted) {
  if (value === "not_requested" || value === "completed" || value === "fallback" || value === "unavailable" || value === "cooldown" || value === "quota") {
    return value;
  }
  if (source === "Configured visual AI") return "completed";
  return visualAiAttempted ? "fallback" : "not_requested";
}

/** @param {unknown} records @param {CaptureActivity} next */
export function appendActivity(records, next) {
  const seen = new Set();
  return [next, ...normalizedActivity(records)]
    .filter((record) => {
      if (seen.has(record.id)) return false;
      seen.add(record.id);
      return true;
    })
    .slice(0, MAX_ACTIVITY_EVENTS);
}

/** @param {unknown} records @param {number} tabId */
export function activityForTab(records, tabId) {
  return normalizedActivity(records).filter((record) => record.tabId === tabId);
}

/** @param {unknown} records @param {number} tabId */
export function withoutActivityForTab(records, tabId) {
  return normalizedActivity(records).filter((record) => record.tabId !== tabId);
}
