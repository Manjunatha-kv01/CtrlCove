// @ts-check

import { sanitizeContext } from "./guide-engine.js";

export const VISUAL_REPEAT_COOLDOWN_MS = 30_000;

/** @typedef {{ fingerprint: string, capturedAt: number }} VisualCaptureTarget */

/** @param {import("./guide-engine.js").BrowserContext} context */
export function visualCaptureFingerprint(context) {
  const sanitized = sanitizeContext(context);
  return JSON.stringify({
    pageUrl: sanitized.pageUrl,
    element: sanitized.element
  });
}

/** @param {VisualCaptureTarget | undefined} previous @param {import("./guide-engine.js").BrowserContext} context @param {number} [now] */
export function visualCaptureDecision(previous, context, now = Date.now()) {
  const fingerprint = visualCaptureFingerprint(context);
  const allowed = !previous
    || previous.fingerprint !== fingerprint
    || now - previous.capturedAt >= VISUAL_REPEAT_COOLDOWN_MS;
  return { allowed, fingerprint };
}
