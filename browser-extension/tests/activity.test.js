import assert from "node:assert/strict";
import test from "node:test";
import { MAX_ACTIVITY_EVENTS, activityForTab, appendActivity, normalizedActivity, withoutActivityForTab } from "../lib/activity.js";

function event(id, tabId = 1) {
  return {
    id,
    tabId,
    pageUrl: "https://example.test/page",
    focus: "Save changes",
    source: "Local context guide",
    visualAiAttempted: false,
    visualAiOutcome: "not_requested",
    capturedAt: "2026-07-19T10:00:00.000Z",
    screenshot: "must-not-be-retained"
  };
}

test("activity normalization preserves metadata only", () => {
  const [record] = normalizedActivity([event("event-1")]);

  assert.equal(record.id, "event-1");
  assert.equal("screenshot" in record, false);
  assert.equal(record.visualAiOutcome, "not_requested");
});

test("activity exposes visual outcomes while upgrading legacy activity safely", () => {
  const [completed, fallback, cooldown, quota] = normalizedActivity([
    { ...event("visual-completed"), source: "Configured visual AI", visualAiAttempted: true, visualAiOutcome: "completed" },
    { ...event("legacy-fallback"), visualAiAttempted: true, visualAiOutcome: undefined },
    { ...event("visual-cooldown"), visualAiOutcome: "cooldown" },
    { ...event("visual-quota"), visualAiOutcome: "quota" }
  ]);

  assert.equal(completed.visualAiOutcome, "completed");
  assert.equal(fallback.visualAiOutcome, "fallback");
  assert.equal(cooldown.visualAiOutcome, "cooldown");
  assert.equal(quota.visualAiOutcome, "quota");
});

test("activity is deduplicated, bounded, and filterable by tab", () => {
  let records = [];
  for (let index = 0; index < MAX_ACTIVITY_EVENTS + 4; index += 1) {
    records = appendActivity(records, event(`event-${index}`, index % 2 === 0 ? 1 : 2));
  }
  records = appendActivity(records, event(`event-${MAX_ACTIVITY_EVENTS + 3}`, 2));

  assert.equal(records.length, MAX_ACTIVITY_EVENTS);
  assert.equal(activityForTab(records, 1).every((record) => record.tabId === 1), true);
  assert.equal(records.filter((record) => record.id === `event-${MAX_ACTIVITY_EVENTS + 3}`).length, 1);
});

test("current-tab activity can be removed without affecting other tabs", () => {
  const records = [event("tab-1", 1), event("tab-2", 2)];
  const remaining = withoutActivityForTab(records, 1);

  assert.deepEqual(remaining.map((record) => record.id), ["tab-2"]);
});
