import assert from "node:assert/strict";
import test from "node:test";
import {
  aiConfigurationEpochIsCurrent,
  captureEpochIsCurrent,
  captureSequenceIsCurrent,
  captureTargetsActiveWindow,
  captureTargetsCurrentDocument,
  captureTargetsCurrentPage
} from "../lib/capture-guard.js";

test("capture guard accepts only the current sanitized page", () => {
  assert.equal(
    captureTargetsCurrentPage("https://example.test/settings?account=private", "https://example.test/settings?token=redacted"),
    true
  );
  assert.equal(captureTargetsCurrentPage("https://example.test/settings", "https://example.test/billing"), false);
  assert.equal(captureTargetsCurrentPage("chrome://settings", "chrome://settings"), false);
});

test("capture guard rejects an async result from a previous navigation epoch", () => {
  assert.equal(captureEpochIsCurrent(4, 4), true);
  assert.equal(captureEpochIsCurrent(5, 4), false);
});

test("capture guard rejects visual work from a previous AI configuration", () => {
  assert.equal(aiConfigurationEpochIsCurrent(3, 3), true);
  assert.equal(aiConfigurationEpochIsCurrent(4, 3), false);
});

test("capture guard accepts only the latest capture sequence", () => {
  assert.equal(captureSequenceIsCurrent(7, 7), true);
  assert.equal(captureSequenceIsCurrent(8, 7), false);
  assert.equal(captureSequenceIsCurrent(undefined, 7), false);
});

test("capture guard rejects a pause from a prior document with the same path", () => {
  assert.equal(captureTargetsCurrentDocument("document-new", "document-new"), true);
  assert.equal(captureTargetsCurrentDocument("document-new", "document-old"), false);
  assert.equal(captureTargetsCurrentDocument(undefined, "document-new"), false);
});

test("capture guard requires the original tab to remain active in its capture window", () => {
  assert.equal(captureTargetsActiveWindow(true, 4, 4), true);
  assert.equal(captureTargetsActiveWindow(false, 4, 4), false);
  assert.equal(captureTargetsActiveWindow(true, 5, 4), false);
});
