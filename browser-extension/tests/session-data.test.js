import assert from "node:assert/strict";
import test from "node:test";
import {
  ACTIVITY_KEY,
  GUIDE_KEY_PREFIX,
  VISUAL_CAPTURE_BUDGETS_KEY,
  captureStorageKeys,
  captureStorageSummary
} from "../lib/session-data.js";

test("capture-data cleanup targets only guides and activity metadata", () => {
  const stored = {
    [ACTIVITY_KEY]: [{ id: "event-1" }],
    [VISUAL_CAPTURE_BUDGETS_KEY]: { "https://example.test": [1] },
    [`${GUIDE_KEY_PREFIX}12`]: { summary: "guide" },
    [`${GUIDE_KEY_PREFIX}38`]: { summary: "guide" },
    "cymos.browser.ai.api-key": "session-secret",
    unrelated: "keep"
  };

  assert.deepEqual(captureStorageKeys(stored), [
    ACTIVITY_KEY,
    VISUAL_CAPTURE_BUDGETS_KEY,
    `${GUIDE_KEY_PREFIX}12`,
    `${GUIDE_KEY_PREFIX}38`
  ]);
  assert.deepEqual(captureStorageSummary(stored, stored[ACTIVITY_KEY]), {
    guideCount: 2,
    activityCount: 1,
    budgetOriginCount: 1
  });
});
