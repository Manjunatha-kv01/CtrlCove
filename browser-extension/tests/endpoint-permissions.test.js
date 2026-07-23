import assert from "node:assert/strict";
import test from "node:test";
import {
  endpointPermissionPattern,
  endpointPermissionsToRevoke,
  hasExactEndpointPermission,
  managedEndpointPermissions,
  normalizeVisualAiEndpoint
} from "../lib/endpoint-permissions.js";

test("managed endpoint permissions retain only precise supported endpoint origins", () => {
  assert.deepEqual(
    managedEndpointPermissions(
      ["https://vision.example.test/*", "https://vision.example.test/*", "https://*/*", "http://example.test/*"],
      "http://localhost:11434/*"
    ),
    ["http://localhost:11434/*", "https://vision.example.test/*"]
  );
});

test("endpoint rotation revokes every managed origin except the retained endpoint", () => {
  assert.deepEqual(
    endpointPermissionsToRevoke(
      ["https://old.example.test/*", "https://new.example.test/*"],
      "https://new.example.test/*"
    ),
    ["https://old.example.test/*"]
  );
});

test("visual AI endpoints are normalized without credentials or URL secrets", () => {
  assert.equal(
    normalizeVisualAiEndpoint("https://vision.example.test/v1/chat/completions"),
    "https://vision.example.test/v1/chat/completions"
  );
  assert.equal(endpointPermissionPattern("https://vision.example.test/v1/chat/completions"), "https://vision.example.test/*");
  assert.throws(() => normalizeVisualAiEndpoint("https://token@example.test/v1"), /credentials/i);
  assert.throws(() => normalizeVisualAiEndpoint("https://example.test/v1?api_key=secret"), /query parameters/i);
  assert.throws(() => normalizeVisualAiEndpoint("http://example.test/v1"), /HTTPS/i);
});

test("configured visual endpoint permission must exactly match the endpoint host and port", () => {
  const endpoint = "https://vision.example.test/v1/chat/completions";

  assert.equal(hasExactEndpointPermission(endpoint, ["https://vision.example.test/*"]), true);
  assert.equal(hasExactEndpointPermission(endpoint, ["https://other.example.test/*"]), false);
  assert.equal(hasExactEndpointPermission(endpoint, ["https://vision.example.test:8443/*"]), false);
  assert.equal(hasExactEndpointPermission("not a URL", ["https://vision.example.test/*"]), false);
});
