import assert from "node:assert/strict";
import test from "node:test";
import { createVisualRequestTimeout } from "../lib/visual-request-timeout.js";

test("visual request timeout aborts only the request signal and records the timeout", async () => {
  const external = new AbortController();
  const timeout = createVisualRequestTimeout(external.signal, 5);

  await new Promise((resolve) => timeout.signal.addEventListener("abort", resolve, { once: true }));

  assert.equal(timeout.signal.aborted, true);
  assert.equal(external.signal.aborted, false);
  assert.equal(timeout.didTimeout(), true);
  timeout.dispose();
});

test("external cancellation aborts the request signal without being counted as a timeout", () => {
  const external = new AbortController();
  const timeout = createVisualRequestTimeout(external.signal, 1_000);

  external.abort();

  assert.equal(timeout.signal.aborted, true);
  assert.equal(timeout.didTimeout(), false);
  timeout.dispose();
});
