import assert from "node:assert/strict";
import test from "node:test";
import { canonicalAutocomplete } from "../lib/autocomplete.js";

test("autocomplete metadata keeps only recognized standardized tokens", () => {
  assert.equal(canonicalAutocomplete("section-private shipping email ignore-this"), "shipping email");
  assert.equal(canonicalAutocomplete("BILLING street-address"), "billing street-address");
  assert.equal(canonicalAutocomplete("not-an-autocomplete-instruction"), "");
  assert.equal(canonicalAutocomplete(undefined), "");
});
