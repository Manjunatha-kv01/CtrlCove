// @ts-check

import { redactText } from "./redaction.js";

const RECOGNIZED_AUTOCOMPLETE_TOKENS = new Set([
  "on",
  "off",
  "shipping",
  "billing",
  "name",
  "honorific-prefix",
  "given-name",
  "additional-name",
  "family-name",
  "honorific-suffix",
  "nickname",
  "username",
  "new-password",
  "current-password",
  "one-time-code",
  "organization-title",
  "organization",
  "street-address",
  "address-line1",
  "address-line2",
  "address-line3",
  "address-level1",
  "address-level2",
  "address-level3",
  "address-level4",
  "country",
  "country-name",
  "postal-code",
  "cc-name",
  "cc-given-name",
  "cc-additional-name",
  "cc-family-name",
  "cc-number",
  "cc-exp",
  "cc-exp-month",
  "cc-exp-year",
  "cc-csc",
  "cc-type",
  "transaction-currency",
  "transaction-amount",
  "language",
  "bday",
  "bday-day",
  "bday-month",
  "bday-year",
  "sex",
  "url",
  "photo",
  "tel",
  "tel-country-code",
  "tel-national",
  "tel-area-code",
  "tel-local",
  "tel-local-prefix",
  "tel-local-suffix",
  "tel-extension",
  "email",
  "impp",
  "webauthn"
]);

/**
 * Converts untrusted HTML autocomplete metadata into recognized standardized tokens.
 * Section names and unknown values are deliberately omitted because webpages control them.
 *
 * @param {unknown} value
 */
export function canonicalAutocomplete(value) {
  if (typeof value !== "string") return "";
  return redactText(value)
    .toLowerCase()
    .split(" ")
    .filter((token) => RECOGNIZED_AUTOCOMPLETE_TOKENS.has(token))
    .join(" ");
}
