import assert from "node:assert/strict";
import test from "node:test";
import { isSafeBrowserContext } from "../lib/browser-context.js";

function context(overrides = {}) {
  return {
    pageUrl: "https://example.test/settings",
    pageTitle: "Settings",
    pageInstanceId: "document-session-123",
    pointer: { x: 144, y: 88, viewportWidth: 1200, viewportHeight: 800 },
    element: {
      tagName: "BUTTON",
      role: "button",
      label: "Save changes",
      text: "Save changes",
      inputType: "",
      bounds: { left: 16, top: 24, width: 180, height: 44 }
    },
    capturedAt: "2026-07-20T10:00:00.000Z",
    ...overrides
  };
}

test("browser context accepts bounded canonical page metadata", () => {
  assert.equal(isSafeBrowserContext(context()), true);
  assert.equal(isSafeBrowserContext(context({ element: { ...context().element, autocomplete: "current-password" } })), true);
});

test("browser context rejects query-bearing URLs, invalid IDs, and unbounded values", () => {
  assert.equal(isSafeBrowserContext(context({ pageUrl: "https://example.test/settings?token=secret" })), false);
  assert.equal(isSafeBrowserContext(context({ pageInstanceId: "document id with spaces" })), false);
  assert.equal(isSafeBrowserContext(context({ pageTitle: "x".repeat(361) })), false);
  assert.equal(isSafeBrowserContext(context({ pointer: { x: Infinity, y: 0, viewportWidth: 1, viewportHeight: 1 } })), false);
  assert.equal(isSafeBrowserContext(context({ element: { ...context().element, autocomplete: "x".repeat(121) } })), false);
  assert.equal(isSafeBrowserContext(context({ capturedAt: "not-a-timestamp" })), false);
});
