# CYMOS Browser Companion

CYMOS Browser Companion is an opt-in Chrome or Chromium Edge extension for contextual browsing guidance. It is separate from the desktop application so browser permissions, visual capture, and any remote AI connection remain deliberate and inspectable.

## What It Does

1. You open the extension side panel and enable the current site.
2. When the pointer pauses over a field, link, or control, the extension captures a small sanitized DOM context: page origin and path, title, cursor position, visible control metadata, and a timestamp.
3. A local rule-based guide updates the side panel. It suggests steps but never clicks, submits, navigates, purchases, or executes actions.
4. Optionally, you can enable visual AI for that site. Only then does the service worker capture the visible tab and send it to your explicitly configured OpenAI-compatible vision endpoint.
5. The screenshot is used in memory for that request only. It is not saved in extension storage or CYMOS.
6. The side panel shows a bounded, session-only capture activity trail. It records the local/visual guide outcome, timestamp, and focus metadata. It never contains screenshots, raw DOM text, API keys, endpoint URLs, AI responses, or request bodies.
7. Select **Save guide to CYMOS** to place a structured, redacted guide on the local clipboard. The running CYMOS desktop app captures it through its existing local clipboard monitor.

## Privacy Model

- No static host permission and no automatic injection across websites.
- Extension pages and the service worker use an explicit Manifest V3 CSP: scripts and styles come only from the packaged extension, frames and form submission are disabled, and remote connections are limited to HTTPS or local development hosts that still require Chrome optional-host permission.
- `activeTab` and `scripting` access are exercised only after a user enables the current site.
- Page URL query strings and fragments are removed before guide processing.
- The service worker accepts cursor context only when it has a canonical HTTP(S) page URL, a bounded document identifier and text fields, finite coordinate ranges, and a valid timestamp. Invalid page-derived messages are ignored before capture processing.
- Common keys, tokens, JWTs, passwords, and private-key blocks are redacted from text context.
- Invisible control characters and bidirectional display overrides are removed from page-derived text before it reaches the guide, clipboard handoff, or optional visual-AI request.
- The remote visual request independently sanitizes its text context again at the network boundary and omits CYMOS's internal document identifier.
- The visual-AI prompt treats the screenshot and every webpage-derived value as untrusted reference data. Instructions embedded in a page must not alter the response contract or be followed.
- CYMOS validates the transient screenshot before remote egress: only JPEG, PNG, or WebP data URLs at or below a 2 MiB raw-image budget are eligible. The final serialized visual-AI request has a separate 3 MiB ceiling that includes image encoding, prompt text, and JSON overhead. Over-budget or malformed captures remain local and fall back to the rule-based guide.
- CYMOS suppresses visual capture before taking a screenshot when the focused control signals password, one-time-code, token, payment, private-key, or similar sensitive entry. It also honors password, OTP, payment, WebAuthn, contact, identity, and address HTML autocomplete hints even when a field is visually a generic text input. Autocomplete metadata is reduced to recognized HTML tokens, so website-defined section names and unknown values never reach guide storage, clipboard handoff, or optional visual-AI context. The local guide remains available; this focused-control check is not a claim that the whole page is secret-free.
- CYMOS also keeps visual capture local on common sensitive-flow URL paths, including sign-in, account, password, billing, payment, checkout, security, verification, and MFA flows. The path policy is intentionally conservative and exact-path-segment based, so documentation such as `/guides/checkout-integration` remains eligible.
- Remote visual responses are streamed through a 256 KiB JSON budget. Oversized, empty, or malformed responses fall back locally and are not retained.
- A remote visual-AI request has a 12-second request-only timeout. Timeouts keep the local guide, count toward the endpoint circuit breaker, and remain distinct from navigation, consent, and configuration cancellation.
- CYMOS reserves a session-only budget of at most 24 visual captures per HTTP(S) origin per rolling hour before it takes a screenshot. The reservation is serialized across tabs and service-worker wake-ups, stores only origin/timestamp metadata, retains at most the 100 most recently used origins, and is removed by **Clear all** or browser restart. When a site reaches its budget, local guidance continues and activity reports the limit.
- The current-site panel displays the remaining visual-capture allowance for the rolling hour. It is a local metadata count only and refreshes after a guide or **Clear all** action.
- When the allowance is exhausted, the panel shows the local time when the oldest rolling-window reservation expires and the next visual capture can proceed.
- Visual requests omit ambient credentials and referrers, bypass HTTP caches, and reject redirects. Responses must declare an `application/json` media type before CYMOS parses them.
- CYMOS accepts a visual guide only when its embedded JSON has a bounded summary, confidence from 0 to 1, and one to five bounded advisory steps. Invalid model output falls back to the local guide.
- Visual capture is disabled by default.
- Remote AI is disabled by default and requires endpoint permission, explicit confirmation, and a session-only API key.
- The API key uses `chrome.storage.session`; it is cleared on browser restart and is never exposed to content scripts or saved in `chrome.storage.local`.
- Session API keys are optional only when a valid key is already present for the current browser session. New keys are bounded to 1024 non-whitespace characters, validated before endpoint permission is requested, and are never stored while visual AI is disabled.
- CYMOS tracks only the exact endpoint origin permission it requests. Changing endpoints revokes stale managed endpoint access; **Clear AI access** disables visual AI, clears the session key, and revokes all managed visual-AI endpoint permissions.
- Before every visual screenshot, CYMOS rechecks both its managed endpoint record and Chrome's live optional-host permission. A permission removed outside CYMOS skips visual capture and leaves the local guide active.
- Because Chrome's visible-tab API is window-scoped, CYMOS checks that the original tab, window, URL, and document are still current immediately before and after capture. A tab switch prevents visual-image egress; the browser API does not offer a fully atomic tab-specific capture.
- Capture activity is held in `chrome.storage.session`, capped at 20 metadata-only events, and cleared when the browser restarts.
- Activity distinguishes local guides, completed visual guides, remote visual fallbacks, visual capture that was unavailable before an attempt, and a visual-AI cooldown. The outcome is metadata only; CYMOS does not retain screenshot bytes or remote error/response data.
- Three consecutive non-aborted visual-AI failures open an in-memory 60-second circuit breaker for that configured endpoint. During cooldown, CYMOS does not take or send screenshots and keeps local guides active. A successful response or AI-configuration change resets the breaker; any AI-configuration change also aborts in-flight visual work and prevents an old remote guide from being stored or shown after the new configuration takes effect.
- When the circuit is open, the current-site status reads **Visual AI cooling down** and refreshes itself when the local cooldown ends. The panel receives only a remaining-duration value, never an endpoint, error body, request, response, or screenshot.
- Capture data is scoped to the active document. Navigating a tab removes its prior guide/activity data, and an asynchronous visual-AI response from the prior page is discarded. An opaque document identifier is held only in session storage to preserve this guard when the Manifest V3 worker wakes again; it is not included in activity entries or CYMOS clipboard handoff.
- Selecting **Disable this site** revokes future pause capture for the current origin and immediately clears the active page's guide/activity data. It also invalidates any visual-AI request that was already in flight.
- Select **Clear page** in the activity panel to remove the current page's guide and activity entries immediately; other tabs remain unchanged.
- The activity panel shows the total session guide, event, and visual-site budget count. Select **Clear all** to remove every stored guide, activity entry, and privacy-budget record immediately; it does not change site consent, visual-AI configuration, or the session-only API key.
- A newer pause, navigation, site disablement, tab closure, **Clear page**, or **Clear all** aborts pending visual work when possible and always discards its result. CYMOS cannot retract bytes already accepted by a remote endpoint.
- Repeated pauses over the same sanitized page target keep producing local guides, but CYMOS suppresses repeat visual-image egress for 30 seconds while its service worker is alive. This in-memory throttle stores no page data and resets on worker restart, navigation, consent revocation, or tab closure.
- When a tab closes, CYMOS removes that tab's guide and activity entries from session storage automatically.
- The browser controls which side the side panel appears on. CYMOS does not force panel placement.

## Install Locally

1. From the CYMOS project root, run `npm install`.
2. Run `npm run check:extension` and `npm run test:extension`.
3. Open `chrome://extensions` in Chrome or `edge://extensions` in Edge.
4. Enable **Developer mode**.
5. Select **Load unpacked** and choose this `browser-extension` folder.
6. Pin **CYMOS Browser Companion**, open a standard HTTP or HTTPS page, and select the extension action to open the side panel.
7. Select **Enable this site**. Pause over an element to generate local guidance.

## Optional Visual AI Contract

The configured endpoint must accept an OpenAI-compatible chat-completions JSON payload with image data URLs and return a JSON object in the assistant message content:

```json
{
  "summary": "Brief context-aware description",
  "confidence": 0.0,
  "steps": ["Suggested step one", "Suggested step two"]
}
```

Use HTTPS for a remote endpoint. HTTP is accepted only for `localhost` or `127.0.0.1` development endpoints. Endpoint access is granted per host and port. The extension fails closed to the local guide if visual capture, endpoint permission, session credentials, or the remote response is unavailable.

Endpoints cannot include URL credentials, query parameters, or fragments. Model identifiers are limited to 120 safe identifier characters (`A-Z`, `a-z`, `0-9`, `.`, `_`, `:`, `/`, and `-`).

## Known Boundary

This release intentionally uses CYMOS's clipboard monitor as the desktop handoff. A future native bridge can replace that explicit save action with an authenticated local desktop channel after its threat model, token rotation, CORS policy, and installer flow are in place.
