// @ts-check

const MAX_TEXT_LENGTH = 360;
const UNSAFE_DISPLAY_CHARACTERS = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F-\u009F\u202A-\u202E\u2066-\u2069]/g;

const SENSITIVE_PATTERNS = [
  /-----BEGIN(?: [A-Z]+)? PRIVATE KEY-----[\s\S]*?-----END(?: [A-Z]+)? PRIVATE KEY-----/gi,
  /\b(?:eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,})\b/g,
  /\b(?:sk|pk|ghp|gho|github_pat)_[a-zA-Z0-9_-]{12,}\b/gi,
  /\b(?:api[_-]?key|access[_-]?token|auth(?:orization)?|bearer|password|secret|token)\s*[:=]\s*[^\s,;]+/gi
];

/** @param {string} value */
export function redactText(value) {
  let redacted = value
    .replace(UNSAFE_DISPLAY_CHARACTERS, "")
    .replace(/\s+/g, " ")
    .trim();
  for (const pattern of SENSITIVE_PATTERNS) {
    redacted = redacted.replace(pattern, "[REDACTED]");
  }
  return redacted.length > MAX_TEXT_LENGTH
    ? `${redacted.slice(0, MAX_TEXT_LENGTH - 3)}...`
    : redacted;
}

/** @param {string} value */
export function safePageUrl(value) {
  try {
    const url = new URL(value);
    if (url.protocol !== "https:" && url.protocol !== "http:") return "";
    return `${url.origin}${url.pathname}`;
  } catch {
    return "";
  }
}

/** @param {string} value */
export function originForUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:" ? url.origin : "";
  } catch {
    return "";
  }
}
