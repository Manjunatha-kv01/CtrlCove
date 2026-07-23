// @ts-check

export const VISUAL_AI_REQUEST_TIMEOUT_MS = 12_000;

/**
 * @typedef {{ signal: AbortSignal, didTimeout: () => boolean, dispose: () => void }} VisualRequestTimeout
 */

/**
 * Creates a request-only abort signal. External cancellation stays distinguishable from an
 * endpoint timeout, allowing the caller to count only genuine endpoint failures.
 * @param {AbortSignal} externalSignal
 * @param {number} [timeoutMs]
 * @returns {VisualRequestTimeout}
 */
export function createVisualRequestTimeout(externalSignal, timeoutMs = VISUAL_AI_REQUEST_TIMEOUT_MS) {
  const controller = new AbortController();
  let timedOut = false;
  const abortForExternalCancellation = () => controller.abort();
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  let timeoutId;

  if (externalSignal.aborted) {
    abortForExternalCancellation();
  } else {
    externalSignal.addEventListener("abort", abortForExternalCancellation, { once: true });
    timeoutId = setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, timeoutMs);
  }

  return {
    signal: controller.signal,
    didTimeout: () => timedOut,
    dispose: () => {
      if (timeoutId !== undefined) clearTimeout(timeoutId);
      externalSignal.removeEventListener("abort", abortForExternalCancellation);
    }
  };
}
