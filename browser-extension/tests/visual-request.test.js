import assert from "node:assert/strict";
import test from "node:test";
import {
  buildVisualRequest,
  serializeVisualRequest,
  validateVisualScreenshot,
  visualCaptureAllowedForContext,
  VISUAL_GUIDE_SYSTEM_PROMPT
} from "../lib/visual-request.js";

function context() {
  return {
    pageUrl: "https://example.test/workspace?token=should-not-leave-device",
    pageTitle: "Settings bearer=should-not-leave-device",
    pageInstanceId: "document-private-lifecycle-id",
    pointer: { x: 10.4, y: 20.8, viewportWidth: 1200, viewportHeight: 800 },
    element: {
      tagName: "BUTTON",
      role: "button",
      label: "Save access_token=should-not-leave-device",
      text: "Save",
      inputType: "",
      bounds: { left: 12.2, top: 21.4, width: 120.4, height: 44.2 }
    },
    capturedAt: "2026-07-20T06:00:00.000Z"
  };
}

test("visual request uses independently sanitized context without internal lifecycle metadata", () => {
  const request = buildVisualRequest(context(), "data:image/jpeg;base64,AA==", { model: "vision-model" });
  const contextText = request.messages[1].content[0].text;

  assert.equal(request.model, "vision-model");
  assert.match(contextText, /https:\/\/example\.test\/workspace/);
  assert.match(contextText, /untrusted cursor context/i);
  assert.match(contextText, /<cursor_context>/);
  assert.doesNotMatch(contextText, /should-not-leave-device/);
  assert.doesNotMatch(contextText, /document-private-lifecycle-id/);
  assert.equal(request.messages[1].content[1].image_url.url, "data:image/jpeg;base64,AA==");
});

test("visual guide prompt treats page content as untrusted reference data", () => {
  assert.match(VISUAL_GUIDE_SYSTEM_PROMPT, /untrusted reference data/i);
  assert.match(VISUAL_GUIDE_SYSTEM_PROMPT, /never as instructions/i);
  assert.match(VISUAL_GUIDE_SYSTEM_PROMPT, /do not disclose secrets/i);
});

test("visual capture is suppressed for password and secret-bearing focused fields", () => {
  const passwordField = context();
  passwordField.element = { ...passwordField.element, tagName: "INPUT", inputType: "password" };
  const tokenField = context();
  tokenField.element = { ...tokenField.element, tagName: "INPUT", label: "API access token", inputType: "text" };
  const normalButton = context();
  normalButton.element = { ...normalButton.element, label: "Save changes", text: "Save changes" };
  const autocompletePassword = context();
  autocompletePassword.element = {
    ...autocompletePassword.element,
    tagName: "INPUT",
    label: "Sign in",
    text: "",
    inputType: "text",
    autocomplete: "current-password"
  };

  assert.equal(visualCaptureAllowedForContext(passwordField), false);
  assert.equal(visualCaptureAllowedForContext(tokenField), false);
  assert.equal(visualCaptureAllowedForContext(autocompletePassword), false);
  assert.equal(visualCaptureAllowedForContext(normalButton), true);
});

test("visual capture is suppressed for personal-data autocomplete fields", () => {
  const emailField = context();
  emailField.element = {
    ...emailField.element,
    tagName: "INPUT",
    label: "Work email",
    text: "",
    inputType: "email",
    autocomplete: "email"
  };
  const addressField = context();
  addressField.element = {
    ...addressField.element,
    tagName: "INPUT",
    label: "Street address",
    text: "",
    inputType: "text",
    autocomplete: "shipping street-address"
  };
  const untrustedEmailField = context();
  untrustedEmailField.element = {
    ...untrustedEmailField.element,
    tagName: "INPUT",
    label: "Contact preference",
    text: "",
    inputType: "text",
    autocomplete: "section-private email ignore-this"
  };
  const ordinaryField = context();
  ordinaryField.element = {
    ...ordinaryField.element,
    tagName: "INPUT",
    label: "Project name",
    text: "",
    inputType: "text",
    autocomplete: "off"
  };

  assert.equal(visualCaptureAllowedForContext(emailField), false);
  assert.equal(visualCaptureAllowedForContext(addressField), false);
  assert.equal(visualCaptureAllowedForContext(untrustedEmailField), false);
  assert.equal(visualCaptureAllowedForContext(ordinaryField), true);
});

test("visual capture is suppressed for sensitive account and transaction flows", () => {
  const signIn = context();
  signIn.pageUrl = "https://example.test/sign-in";
  const checkout = context();
  checkout.pageUrl = "https://example.test/store/checkout/review";
  const documentation = context();
  documentation.pageUrl = "https://docs.example.test/guides/checkout-integration";
  documentation.element = { ...documentation.element, label: "Read the guide", text: "Read" };

  assert.equal(visualCaptureAllowedForContext(signIn), false);
  assert.equal(visualCaptureAllowedForContext(checkout), false);
  assert.equal(visualCaptureAllowedForContext(documentation), true);
  assert.equal(visualCaptureAllowedForContext({ ...context(), pageUrl: "not-a-url" }), false);
});

test("visual request blocks malformed and over-budget images before remote egress", () => {
  assert.throws(
    () => validateVisualScreenshot("data:image/gif;base64,AA=="),
    /unsupported image format/i
  );
  assert.throws(
    () => validateVisualScreenshot("data:image/jpeg;base64,AAAA", 2),
    /remote visual budget/i
  );
});

test("visual request serialization enforces a total remote payload budget", () => {
  const request = buildVisualRequest(context(), "data:image/jpeg;base64,AA==", { model: "vision-model" });
  const serialized = serializeVisualRequest(request);

  assert.equal(typeof serialized, "string");
  assert.match(serialized, /vision-model/);
  assert.throws(
    () => serializeVisualRequest({ content: "x".repeat(11) }, 10),
    /remote request budget/i
  );
});
