import assert from "node:assert/strict";
import test from "node:test";
import {
  MAX_CONSECUTIVE_VISUAL_FAILURES,
  VISUAL_FAILURE_COOLDOWN_MS,
  nextVisualCircuitFailure,
  visualCircuitCooldownRemaining,
  visualCircuitDecision
} from "../lib/visual-circuit-breaker.js";

test("visual circuit opens after consecutive remote failures", () => {
  let state;
  for (let index = 0; index < MAX_CONSECUTIVE_VISUAL_FAILURES; index += 1) {
    state = nextVisualCircuitFailure(state, 10_000 + index);
  }

  assert.equal(visualCircuitDecision(state, 10_100).allowed, false);
  assert.equal(visualCircuitCooldownRemaining(state, 10_100), 59_902);
  assert.equal(visualCircuitDecision(state, 10_002 + VISUAL_FAILURE_COOLDOWN_MS).allowed, true);
  assert.equal(visualCircuitCooldownRemaining(state, 10_002 + VISUAL_FAILURE_COOLDOWN_MS), 0);
});

test("visual circuit retains retries before the failure threshold", () => {
  const state = nextVisualCircuitFailure(undefined, 10_000);

  assert.equal(state.consecutiveFailures, 1);
  assert.equal(visualCircuitDecision(state, 10_001).allowed, true);
});
