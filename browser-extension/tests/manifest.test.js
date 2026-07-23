import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const manifest = JSON.parse(
  await readFile(new URL("../manifest.json", import.meta.url), "utf8")
);

test("manifest declares a restrictive MV3 extension-page CSP", () => {
  const policy = manifest.content_security_policy?.extension_pages;

  assert.equal(typeof policy, "string");
  assert.match(policy, /script-src 'self'/);
  assert.match(policy, /object-src 'self'/);
  assert.match(policy, /base-uri 'none'/);
  assert.match(policy, /frame-src 'none'/);
  assert.match(policy, /form-action 'none'/);
  assert.match(policy, /connect-src 'self' https: http:\/\/localhost:\* http:\/\/127\.0\.0\.1:\*/);
});
