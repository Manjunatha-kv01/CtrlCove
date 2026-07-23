import assert from "node:assert/strict";
import test from "node:test";
import { normalizeVisualAiApiKey, normalizeVisualAiModel } from "../lib/ai-configuration.js";

test("visual AI model identifiers accept bounded provider-compatible names", () => {
  assert.equal(normalizeVisualAiModel("Qwen/Qwen2.5-VL-7B-Instruct"), "Qwen/Qwen2.5-VL-7B-Instruct");
  assert.equal(normalizeVisualAiModel("gpt-4o-mini"), "gpt-4o-mini");
});

test("visual AI model identifiers reject empty, oversized, and unsafe values", () => {
  assert.throws(() => normalizeVisualAiModel("   "), /model name/i);
  assert.throws(() => normalizeVisualAiModel("model name"), /safe identifier/i);
  assert.throws(() => normalizeVisualAiModel("a".repeat(121)), /120 characters/i);
});

test("visual AI session keys are bounded header-safe values", () => {
  assert.equal(normalizeVisualAiApiKey("sk-local-example_123"), "sk-local-example_123");
  assert.throws(() => normalizeVisualAiApiKey(" key"), /whitespace/i);
  assert.throws(() => normalizeVisualAiApiKey("key\nnext"), /whitespace/i);
  assert.throws(() => normalizeVisualAiApiKey("a".repeat(1025)), /1024 characters/i);
});
