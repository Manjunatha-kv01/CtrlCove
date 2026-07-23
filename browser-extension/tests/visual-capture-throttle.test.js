import assert from "node:assert/strict";
import test from "node:test";
import {
  VISUAL_REPEAT_COOLDOWN_MS,
  visualCaptureDecision
} from "../lib/visual-capture-throttle.js";

function context(overrides = {}) {
  return {
    pageUrl: "https://example.test/settings",
    pageTitle: "Settings",
    pageInstanceId: "document-session-123",
    pointer: { x: 10, y: 20, viewportWidth: 1200, viewportHeight: 800 },
    element: {
      tagName: "BUTTON",
      role: "button",
      label: "Save changes",
      text: "Save changes",
      inputType: "",
      bounds: { left: 12, top: 21, width: 120, height: 44 }
    },
    capturedAt: "2026-07-20T10:00:00.000Z",
    ...overrides
  };
}

test("visual capture suppresses a repeated target inside the in-memory cooldown", () => {
  const initial = visualCaptureDecision(undefined, context(), 10_000);
  const repeated = visualCaptureDecision(
    { fingerprint: initial.fingerprint, capturedAt: 10_000 },
    context(),
    10_000 + VISUAL_REPEAT_COOLDOWN_MS - 1
  );

  assert.equal(initial.allowed, true);
  assert.equal(repeated.allowed, false);
});

test("visual capture allows a changed target or an expired cooldown", () => {
  const initial = visualCaptureDecision(undefined, context(), 10_000);
  const changedTarget = visualCaptureDecision(
    { fingerprint: initial.fingerprint, capturedAt: 10_000 },
    context({ element: { ...context().element, label: "Delete draft", text: "Delete draft" } }),
    10_001
  );
  const expired = visualCaptureDecision(
    { fingerprint: initial.fingerprint, capturedAt: 10_000 },
    context(),
    10_000 + VISUAL_REPEAT_COOLDOWN_MS
  );

  assert.equal(changedTarget.allowed, true);
  assert.equal(expired.allowed, true);
});
