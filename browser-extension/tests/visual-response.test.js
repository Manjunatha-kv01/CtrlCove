import assert from "node:assert/strict";
import test from "node:test";
import { parseBoundedVisualResponse, parseVisualGuideContent } from "../lib/visual-response.js";

test("visual response parser accepts bounded JSON", async () => {
  const response = new Response(JSON.stringify({ choices: [{ message: { content: "{}" } }] }), {
    headers: { "content-type": "application/json; charset=utf-8" }
  });

  assert.deepEqual(await parseBoundedVisualResponse(response), {
    choices: [{ message: { content: "{}" } }]
  });
});

test("visual response parser rejects declared and streamed over-budget bodies", async () => {
  const declaredOversize = new Response("{}", {
    headers: { "content-type": "application/json", "content-length": "999" }
  });
  const streamedOversize = new Response("123456", {
    headers: { "content-type": "application/json" }
  });

  await assert.rejects(() => parseBoundedVisualResponse(declaredOversize, 10), /response budget/i);
  await assert.rejects(() => parseBoundedVisualResponse(streamedOversize, 5), /response budget/i);
});

test("visual response parser rejects non-JSON media types", async () => {
  const response = new Response('{"summary":"not accepted"}', {
    headers: { "content-type": "text/plain" }
  });

  await assert.rejects(() => parseBoundedVisualResponse(response), /application\/json/i);
});

test("visual guide content accepts the bounded advisory response contract", () => {
  const guide = parseVisualGuideContent({
    choices: [{
      message: {
        content: JSON.stringify({
          summary: "Review the form before continuing.",
          confidence: 0.8,
          steps: ["Review the label.", "Confirm the current page state."]
        })
      }
    }]
  });

  assert.equal(guide.summary, "Review the form before continuing.");
  assert.equal(guide.steps.length, 2);
});

test("visual guide content rejects invalid schema and oversized embedded guides", () => {
  const invalidConfidence = {
    choices: [{ message: { content: JSON.stringify({ summary: "Review", confidence: 2, steps: ["Check"] }) } }]
  };
  const oversizedGuide = {
    choices: [{ message: { content: JSON.stringify({ summary: "Review", confidence: 0.5, steps: ["Check"] }) } }]
  };

  assert.throws(() => parseVisualGuideContent(invalidConfidence), /confidence/i);
  assert.throws(() => parseVisualGuideContent(oversizedGuide, 10), /guide budget/i);
});
