// @ts-check

import { safePageUrl } from "./redaction.js";

const MAX_PAGE_URL_LENGTH = 2_048;
const MAX_CONTEXT_TEXT_LENGTH = 360;
const MAX_ELEMENT_IDENTIFIER_LENGTH = 120;
const MAX_COORDINATE = 1_000_000;

/** @param {unknown} context */
export function isSafeBrowserContext(context) {
  if (!context || typeof context !== "object" || Array.isArray(context)) return false;
  const candidate = /** @type {{ pageUrl?: unknown, pageTitle?: unknown, pageInstanceId?: unknown, pointer?: unknown, element?: unknown, capturedAt?: unknown }} */ (context);
  if (!isCanonicalPageUrl(candidate.pageUrl) || !isBoundedString(candidate.pageTitle, MAX_CONTEXT_TEXT_LENGTH)) return false;
  if (!isPageInstanceId(candidate.pageInstanceId) || !isIsoTimestamp(candidate.capturedAt)) return false;
  if (!isSafePointer(candidate.pointer) || !isSafeElement(candidate.element)) return false;
  return true;
}

/** @param {unknown} value */
function isCanonicalPageUrl(value) {
  return typeof value === "string"
    && value.length > 0
    && value.length <= MAX_PAGE_URL_LENGTH
    && safePageUrl(value) === value;
}

/** @param {unknown} value @param {number} maxLength */
function isBoundedString(value, maxLength) {
  return typeof value === "string" && value.length <= maxLength;
}

/** @param {unknown} value */
function isPageInstanceId(value) {
  return typeof value === "string" && /^[a-zA-Z0-9_-]{1,64}$/.test(value);
}

/** @param {unknown} value */
function isIsoTimestamp(value) {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

/** @param {unknown} pointer */
function isSafePointer(pointer) {
  if (!pointer || typeof pointer !== "object" || Array.isArray(pointer)) return false;
  const candidate = /** @type {{ x?: unknown, y?: unknown, viewportWidth?: unknown, viewportHeight?: unknown }} */ (pointer);
  return isBoundedNumber(candidate.x, 0)
    && isBoundedNumber(candidate.y, 0)
    && isBoundedNumber(candidate.viewportWidth, 0)
    && isBoundedNumber(candidate.viewportHeight, 0);
}

/** @param {unknown} element */
function isSafeElement(element) {
  if (!element || typeof element !== "object" || Array.isArray(element)) return false;
  const candidate = /** @type {{ tagName?: unknown, role?: unknown, label?: unknown, text?: unknown, inputType?: unknown, autocomplete?: unknown, bounds?: unknown }} */ (element);
  return isBoundedString(candidate.tagName, MAX_ELEMENT_IDENTIFIER_LENGTH)
    && isBoundedString(candidate.role, MAX_ELEMENT_IDENTIFIER_LENGTH)
    && isBoundedString(candidate.label, MAX_CONTEXT_TEXT_LENGTH)
    && isBoundedString(candidate.text, MAX_CONTEXT_TEXT_LENGTH)
    && isBoundedString(candidate.inputType, MAX_ELEMENT_IDENTIFIER_LENGTH)
    && (candidate.autocomplete === undefined || isBoundedString(candidate.autocomplete, MAX_ELEMENT_IDENTIFIER_LENGTH))
    && isSafeBounds(candidate.bounds);
}

/** @param {unknown} bounds */
function isSafeBounds(bounds) {
  if (!bounds || typeof bounds !== "object" || Array.isArray(bounds)) return false;
  const candidate = /** @type {{ left?: unknown, top?: unknown, width?: unknown, height?: unknown }} */ (bounds);
  return isBoundedNumber(candidate.left, -MAX_COORDINATE)
    && isBoundedNumber(candidate.top, -MAX_COORDINATE)
    && isBoundedNumber(candidate.width, 0)
    && isBoundedNumber(candidate.height, 0);
}

/** @param {unknown} value @param {number} minimum */
function isBoundedNumber(value, minimum) {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= MAX_COORDINATE;
}
