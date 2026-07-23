// @ts-check

export const MAX_CONSECUTIVE_VISUAL_FAILURES = 3;
export const VISUAL_FAILURE_COOLDOWN_MS = 60_000;

/** @typedef {{ consecutiveFailures: number, blockedUntil: number }} VisualCircuitState */

/** @param {VisualCircuitState | undefined} state @param {number} [now] */
export function visualCircuitDecision(state, now = Date.now()) {
  return { allowed: !state || state.blockedUntil <= now };
}

/** @param {VisualCircuitState | undefined} state @param {number} [now] */
export function visualCircuitCooldownRemaining(state, now = Date.now()) {
  return Math.max(0, (state?.blockedUntil ?? 0) - now);
}

/** @param {VisualCircuitState | undefined} state @param {number} [now] @returns {VisualCircuitState} */
export function nextVisualCircuitFailure(state, now = Date.now()) {
  const consecutiveFailures = (state?.consecutiveFailures ?? 0) + 1;
  if (consecutiveFailures >= MAX_CONSECUTIVE_VISUAL_FAILURES) {
    return { consecutiveFailures: 0, blockedUntil: now + VISUAL_FAILURE_COOLDOWN_MS };
  }
  return { consecutiveFailures, blockedUntil: 0 };
}
