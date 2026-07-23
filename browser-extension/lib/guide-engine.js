// @ts-check

import { redactText, safePageUrl } from "./redaction.js";
import { canonicalAutocomplete } from "./autocomplete.js";

const MAX_STEPS = 5;

/**
 * @typedef {{ x: number, y: number, viewportWidth: number, viewportHeight: number }} Pointer
 * @typedef {{ left: number, top: number, width: number, height: number }} Bounds
 * @typedef {{ tagName: string, role: string, label: string, text: string, inputType: string, autocomplete?: string, bounds: Bounds }} FocusedElement
 * @typedef {{ pageUrl: string, pageTitle: string, pageInstanceId: string, pointer: Pointer, element: FocusedElement, capturedAt: string }} BrowserContext
 * @typedef {{ summary: string, confidence: number, source: string, steps: string[], context: BrowserContext }} BrowserGuide
 */

/** @param {BrowserContext} context */
export function sanitizeContext(context) {
  return {
    pageUrl: safePageUrl(context.pageUrl),
    pageTitle: redactText(context.pageTitle),
    pageInstanceId: safePageInstanceId(context.pageInstanceId),
    pointer: {
      x: Math.max(0, Math.round(context.pointer.x)),
      y: Math.max(0, Math.round(context.pointer.y)),
      viewportWidth: Math.max(0, Math.round(context.pointer.viewportWidth)),
      viewportHeight: Math.max(0, Math.round(context.pointer.viewportHeight))
    },
    element: {
      tagName: redactText(context.element.tagName).toLowerCase(),
      role: redactText(context.element.role).toLowerCase(),
      label: redactText(context.element.label),
      text: redactText(context.element.text),
      inputType: redactText(context.element.inputType).toLowerCase(),
      autocomplete: canonicalAutocomplete(context.element.autocomplete),
      bounds: {
        left: Math.round(context.element.bounds.left),
        top: Math.round(context.element.bounds.top),
        width: Math.max(0, Math.round(context.element.bounds.width)),
        height: Math.max(0, Math.round(context.element.bounds.height))
      }
    },
    capturedAt: context.capturedAt
  };
}

/** @param {unknown} value */
function safePageInstanceId(value) {
  return typeof value === "string" && /^[a-zA-Z0-9_-]{1,64}$/.test(value) ? value : "";
}

/** @param {BrowserContext} rawContext */
export function buildLocalGuide(rawContext) {
  const context = sanitizeContext(rawContext);
  const subject = context.element.label || context.element.text || context.element.role || context.element.tagName || "this page area";
  const control = `${context.element.tagName} ${context.element.role} ${context.element.inputType}`;
  /** @type {string[]} */
  let steps;
  let summary;

  if (/input|textarea|textbox|combobox|select/.test(control)) {
    summary = `You paused on ${subject}. Review the field before entering information.`;
    steps = [
      "Confirm the field label and the information it expects.",
      "Do not enter passwords, tokens, or private keys unless the site is trusted.",
      "Review validation feedback before continuing."
    ];
  } else if (/button|submit|checkbox|radio|switch/.test(control)) {
    summary = `You paused on ${subject}. This appears to be an action control.`;
    steps = [
      "Review the action label and nearby details.",
      "Confirm the current page state is correct.",
      "Choose the action only when the result is expected."
    ];
  } else if (/a|link|tab/.test(control)) {
    summary = `You paused on ${subject}. This appears to navigate or change the current view.`;
    steps = [
      "Review the destination or section label.",
      "Keep the current task in context before navigating.",
      "Capture a note in CYMOS when this reference is useful."
    ];
  } else {
    summary = `You paused on ${subject}. CYMOS identified page context but will not infer an action without a clearer control.`;
    steps = [
      "Review the nearby labels and page state.",
      "Move over a specific field, link, or control for a focused guide.",
      "Save this guide to CYMOS only when it is useful."
    ];
  }

  return {
    summary,
    confidence: 0.35,
    source: "Local context guide",
    steps: steps.slice(0, MAX_STEPS),
    context
  };
}

/**
 * @param {unknown} response
 * @param {BrowserContext} rawContext
 * @returns {BrowserGuide}
 */
export function normalizeVisionGuide(response, rawContext) {
  const fallback = buildLocalGuide(rawContext);
  if (!response || typeof response !== "object" || Array.isArray(response)) return fallback;
  const candidate = /** @type {{ summary?: unknown, confidence?: unknown, steps?: unknown }} */ (response);
  if (typeof candidate.summary !== "string" || !Array.isArray(candidate.steps)) return fallback;
  const steps = candidate.steps
    .filter((step) => typeof step === "string")
    .map((step) => redactText(step.trim()))
    .filter(Boolean)
    .slice(0, MAX_STEPS);
  if (steps.length === 0) return fallback;

  return {
    summary: redactText(candidate.summary),
    confidence: typeof candidate.confidence === "number"
      ? Math.min(1, Math.max(0, candidate.confidence))
      : 0.6,
    source: "Configured visual AI",
    steps,
    context: sanitizeContext(rawContext)
  };
}

/** @param {BrowserGuide} guide */
export function guideToCymosMarkdown(guide) {
  const lines = [
    "# CYMOS Browser Companion Guide",
    "",
    `- Source: ${guide.source}`,
    `- Page: ${guide.context.pageUrl || "Unavailable"}`,
    `- Focus: ${guide.context.element.label || guide.context.element.text || guide.context.element.tagName}`,
    `- Captured: ${guide.context.capturedAt}`,
    "",
    "## Suggested next steps",
    "",
    ...guide.steps.map((step, index) => `${index + 1}. ${step}`),
    "",
    "## Context",
    "",
    guide.summary
  ];
  return lines.join("\n");
}
