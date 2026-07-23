import assert from "node:assert/strict";
import test from "node:test";
import {
  MAX_VISUAL_CAPTURES_PER_ORIGIN,
  MAX_TRACKED_VISUAL_CAPTURE_ORIGINS,
  VISUAL_CAPTURE_BUDGET_WINDOW_MS,
  normalizeVisualCaptureBudgets,
  reserveVisualCaptureBudget,
  visualCaptureBudgetNextAvailableAt,
  visualCaptureBudgetRemaining,
  visualCaptureBudgetDecision
} from "../lib/visual-capture-budget.js";

test("visual capture budget permits reservations until the per-origin hourly limit", () => {
  const now = 1_000_000;
  const previous = Array.from({ length: MAX_VISUAL_CAPTURES_PER_ORIGIN - 1 }, (_, index) => now - index);
  const decision = visualCaptureBudgetDecision(previous, now);
  const reserved = reserveVisualCaptureBudget(decision, now);

  assert.equal(decision.allowed, true);
  assert.equal(visualCaptureBudgetRemaining(decision), 1);
  assert.equal(visualCaptureBudgetDecision(reserved, now).allowed, false);
});

test("an exhausted visual capture budget reports when the oldest allowance returns", () => {
  const now = 1_000_000;
  const firstAvailableAt = now + 120;
  const exhausted = visualCaptureBudgetDecision(
    Array.from(
      { length: MAX_VISUAL_CAPTURES_PER_ORIGIN },
      (_, index) => firstAvailableAt - VISUAL_CAPTURE_BUDGET_WINDOW_MS + index
    ),
    now
  );

  assert.equal(exhausted.allowed, false);
  assert.equal(visualCaptureBudgetNextAvailableAt(exhausted), firstAvailableAt);
});

test("visual capture budget expires old entries and keeps only valid HTTP origins", () => {
  const now = 1_000_000;
  const budgets = normalizeVisualCaptureBudgets({
    "https://example.test": [now - VISUAL_CAPTURE_BUDGET_WINDOW_MS, now - 1],
    "not-an-origin": [now],
    "https://second.test": ["invalid"]
  }, now);

  assert.deepEqual(budgets, { "https://example.test": [now - 1] });
  assert.equal(visualCaptureBudgetDecision(budgets["https://example.test"], now).allowed, true);
});

test("visual capture budget retains only the most recently used origins", () => {
  const now = 1_000_000;
  const records = Object.fromEntries(
    Array.from(
      { length: MAX_TRACKED_VISUAL_CAPTURE_ORIGINS + 1 },
      (_, index) => [`https://site-${index}.test`, [now - index]]
    )
  );
  const budgets = normalizeVisualCaptureBudgets(records, now);

  assert.equal(Object.keys(budgets).length, MAX_TRACKED_VISUAL_CAPTURE_ORIGINS);
  assert.ok(budgets["https://site-0.test"]);
  assert.equal(budgets["https://site-100.test"], undefined);
});
