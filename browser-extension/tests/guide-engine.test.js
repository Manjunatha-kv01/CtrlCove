import assert from "node:assert/strict";
import test from "node:test";
import {
  buildLocalGuide,
  guideToCymosMarkdown,
  normalizeVisionGuide,
  sanitizeContext
} from "../lib/guide-engine.js";
import { redactText } from "../lib/redaction.js";

function context(overrides = {}) {
  return {
    pageUrl: "https://example.test/checkout?token=should-not-appear",
    pageTitle: "Checkout password=should-not-appear",
    pageInstanceId: "document-session-123",
    pointer: { x: 144.4, y: 88.2, viewportWidth: 1200, viewportHeight: 800 },
    element: {
      tagName: "BUTTON",
      role: "button",
      label: "Place order",
      text: "Place order",
      inputType: "",
      bounds: { left: 16.2, top: 24.8, width: 180.1, height: 44.9 }
    },
    capturedAt: "2026-07-19T10:00:00.000Z",
    ...overrides
  };
}

test("sanitized browser context removes query strings and likely secrets", () => {
  const sanitized = sanitizeContext(context());

  assert.equal(sanitized.pageUrl, "https://example.test/checkout");
  assert.equal(sanitized.pageInstanceId, "document-session-123");
  assert.match(sanitized.pageTitle, /\[REDACTED\]/);
  assert.equal(sanitized.pointer.x, 144);
  assert.equal(sanitized.element.bounds.height, 45);
});

test("sanitized browser context canonicalizes untrusted autocomplete metadata", () => {
  const sanitized = sanitizeContext(context({
    element: {
      ...context().element,
      autocomplete: "section-private shipping email ignore-this"
    }
  }));

  assert.equal(sanitized.element.autocomplete, "shipping email");
});

test("redaction removes invisible control and bidirectional display characters", () => {
  const redacted = redactText("Deploy\u0000 \u202Econfig\u202C now\u2066");

  assert.equal(redacted, "Deploy config now");
  assert.doesNotMatch(redacted, /[\u0000-\u001F\u007F-\u009F\u202A-\u202E\u2066-\u2069]/);
});

test("local guide remains advisory and produces a CYMOS-safe capture", () => {
  const guide = buildLocalGuide(context());
  const markdown = guideToCymosMarkdown(guide);

  assert.equal(guide.source, "Local context guide");
  assert.equal(guide.steps.length, 3);
  assert.match(guide.summary, /action control/i);
  assert.match(markdown, /CYMOS Browser Companion Guide/);
  assert.doesNotMatch(markdown, /should-not-appear/);
});

test("visual AI guide normalization bounds and sanitizes suggested steps", () => {
  const guide = normalizeVisionGuide({
    summary: "Review authorization: Bearer secret-value",
    confidence: 2,
    steps: ["One", "Two", "Three", "Four", "Five", "Six", 7]
  }, context());

  assert.equal(guide.source, "Configured visual AI");
  assert.equal(guide.confidence, 1);
  assert.equal(guide.steps.length, 5);
  assert.match(guide.summary, /\[REDACTED\]/);
});
