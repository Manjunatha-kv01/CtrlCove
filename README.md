# CYMOS

## Control Your Memory Operating System

CYMOS is a local-first desktop memory vault for operational knowledge. It captures useful clipboard content, stores it in SQLite on your device, and turns commands, errors, notes, incidents, fixes, and project timelines into reusable local context.

> Capture what happened. Keep what worked. Find it when it matters.

## What Works Today

### Local Memory Vault

- Clipboard monitoring for text, URLs, code, copied images, and file-oriented content.
- Local SQLite storage with timestamps, content types, tags, source context, search, favorites, collections, and export.
- Hash-based deduplication keeps one canonical memory for repeated copies while recording its copy count and latest use.
- Vault lifecycle controls for retention age, memory count, storage budget, and favorite protection. The default policy retains 365 days, 10,000 memories, and 1 GB locally.
- Typed command-boundary failures with structured local error logging and bounded retry for transient SQLite lock contention.
- Fast local retrieval from the Memory vault.
- Explicit browser bookmark capture from the Memory vault: save a URL with an optional title and tags. CYMOS does not observe browser history, tabs, or browsing activity.
- Optional CYMOS Browser Companion extension for Chrome and Chromium Edge: enable one current site at a time, receive local pause-based guidance in the browser side panel, and explicitly save a redacted guide into the CYMOS clipboard vault. It has no static host permission, screenshot persistence, or remote AI by default.
- Explicit IDE snippet capture from the Memory vault: save code with optional project, file path, language, title, and tags. CYMOS does not inspect editor buffers, repositories, or files in the background.

### Privacy and Reliability

- Sensitive-value protection for private keys, tokens, API keys, JWTs, authorization headers, and common secret assignments.
- Blocked captures are never stored; only a generic local privacy audit reason is retained.
- SQLite WAL mode, integrity checks, foreign-key checks, verified manual backups, and automated snapshot retention.
- Automatic and user-triggered vault maintenance removes expired or over-limit unprotected memories, safely clears dependent links, deletes owned image assets, and refreshes the local knowledge graph.

### Operational Knowledge

- Explicit Bash and Zsh history import with a user-selected newest-command window (100 to 1,000 entries), plus one-at-a-time command capture with optional host, project, and tags. Terminal sessions are not observed automatically.
- Explicit text-based document, log, configuration, and script import into the active workspace session. CYMOS never scans local files in the background.
- InsightTrail timeline for clipboard captures, terminal commands, screenshots, errors, and manual notes.
- Incident grouping for recurring error-shaped captures.
- Deliberate incident evidence links: attach an unassigned saved command, log, file, screenshot, or code capture from a workspace timeline to the incident it supports.
- Local retention controls and source exclusions.

### Workspaces and Sessions

- Named workspaces for separate projects, incidents, or learning tracks.
- One active capture session at a time. New captures are linked to that session automatically.
- Project-scoped timelines, session filters, Memory Replay, incident views, and local pattern signals.
- Archive completed workspaces without deleting their sessions, incidents, remedies, or reports; restore them when the project resumes.
- Existing timeline history is migrated into the default workspace on first launch.

### Runbooks and Reporting

- Record the command or remediation that resolved an incident, along with the outcome.
- Reopen a resolved incident with a follow-up reason; earlier remedies remain in immutable local history.
- Surface a known local remedy when the same incident recurs in another workspace, with the original workspace shown as provenance.
- Search the Local runbook by incident, command, outcome, or source workspace.
- Create standalone local runbooks for SOPs, maintenance procedures, and trusted command sequences before an incident occurs.
- Export a selected runbook as a local Markdown file with revision and review state for a change record, review, or approved sharing workflow.
- Preserve immutable revision history for standalone runbooks, with deliberate restore-as-new-version recovery, so procedure edits remain locally auditable.
- Mark the latest standalone runbook revision as reviewed; edits and recoveries automatically require another review.
- Use the local 90-day review cadence to surface SOPs whose current validation has expired.
- Record optional review evidence with the exact SOP revision; the note remains local and is included in a deliberate Markdown export.
- Copy a selected runbook procedure directly to the system clipboard from the Local runbook, with a local audit record.
- Review the latest bounded Local runbook audit trail for create, update, review, restore, export, and copy activity.
- Filter the Local runbook into reviewed procedures and a focused review queue for changed SOPs.
- Open Workspace directly into the local runbook review queue, then switch filters when investigating an SOP.
- Export a workspace or filtered session as a local Markdown operational report containing events, incidents, and recorded fixes.
- Export a workspace or filtered session as a structured local JSON handoff package for deliberate sharing or downstream tooling.
- Review a workspace handoff locally before export. CYMOS scans the exact selected JSON payload for sensitive patterns and verifies it remains within the 1 MB local import limit; blocked packages are never written.
- Build a selective JSON handoff by excluding individual timeline events from the current workspace or session scope before review; excluded events are omitted from the package and recorded only as a count in the local audit metadata.
- Require an explicit local handoff declaration with recipient, purpose, and handling classification (`Internal`, `Restricted`, or `Confidential`) before export. Declarations are privacy-checked, embedded in the package, and retained as local provenance after verified import.
- Apply a local handoff lifecycle policy of 1 day, 7 days, 30 days, or no expiry. Confidential handoffs must use a timed expiry. Expired packages remain inspectable for audit but cannot be imported.
- Require local trusted-recipient approval before exporting a handoff. Each recipient has a maximum approved handling level, local use metadata, revocation state, and a visible registry picker in the handoff workflow.
- Sign newly exported handoff packages with a device-local Ed25519 key. CYMOS verifies the signature before import; unsigned legacy packages are clearly labeled for compatibility, but confidential packages must be signed and come from a locally trusted signer before import. Verified package signers can be trusted or revoked from the verification panel, which also shows the local trusted-signer registry.
- Record every handoff verification attempt in a metadata-only local ledger with status, signer fingerprint, checksums, size, and rejection reason when applicable. The ledger is automatically bounded to the latest 200 records.
- Keep a local handoff export ledger with scope, declaration, expiry, signer fingerprint, included and excluded counts, package size, SHA-256 checksum, and timestamp. The ledger never stores an export payload or sends it anywhere, and is automatically bounded to the latest 200 records.
- Verify a selected handoff package locally with its SHA-256 payload checksum and package signature, then deliberately import it as an isolated local workspace when needed.
- Configure a local team sharing policy before any future synchronization is enabled. The policy stores allowed sharing scopes, mode, approval gates, and shared-data retention locally; CYMOS still performs no remote sync in this release.
- Run a local team sharing readiness preflight that checks policy state, approved devices, trusted recipients, trusted signers, allowed scopes, and blockers before synchronization work begins.
- Register, approve, and revoke local sharing devices. Approved devices satisfy the local readiness gate, but they do not create a network connection or sync data in this release.
- Review a focused local team sharing audit stream for policy changes and device registration, approval, or revocation decisions.
- Export a local Markdown team sharing readiness report containing policy settings, readiness blockers, device states, and recent sharing audit events.
- Keep the focused team sharing audit stream bounded to the latest 200 metadata records so sharing governance history remains local and controlled.
- Generate a local sync dry-run manifest that estimates eligible devices, scopes, records, bytes, and blockers without sending data or opening a network connection.
- Export the local sync dry-run manifest as JSON for review or downstream tooling. The manifest is generated locally and explicitly records that remote sync is not enabled.
- Inspect exported sync dry-run manifests locally to verify schema, format, estimates, blockers, device count, SHA-256 dry-run checksum, device-local Ed25519 signature, signer fingerprint, local signer trust status, and the required `remote_sync_enabled: false` safety declaration.
- Trust the current CYMOS device signer locally so signed team sharing dry-run manifests can pass both authenticity and local trust checks.
- Review a focused manifest inspection ledger that separates dry-run export and verification decisions from broader team sharing policy and device audit events.
- Export the manifest inspection ledger as a metadata-only Markdown report for offline review without embedding dry-run manifest payloads.
- View a local manifest ledger risk summary with event, verified, warning, and export counts before reviewing individual audit entries.
- Filter the local manifest ledger to warning-only events for quick review of rejected or risky dry-run manifest checks.
- Search the local manifest ledger by signer, checksum, status, actor, timestamp, or exported path without exposing manifest payload data.
- Expand or collapse the local manifest ledger in small batches so longer audit histories remain readable in the Platform view.
- Open individual manifest ledger events to inspect full metadata details while keeping manifest payloads out of the UI.
- Copy a single manifest ledger event as metadata-only text for offline notes or audit review.
- Copy the currently visible manifest ledger results as metadata-only text after search, warning filter, and show-more controls are applied.
- Reset manifest ledger filtering, search, paging, and expanded event details to the default local view with one control.
- Switch the local manifest ledger between all events, verified inspections, warnings, and exports using structured audit metadata.
- Export the manifest ledger entries matching the active local view as a metadata-only Markdown report; the selected filter and search state are recorded, while the raw search field is not recorded as report context.
- Record local filtered-ledger exports with their path, selected filter, and matching-event count, without recording the search phrase.
- Load the dedicated manifest ledger from its full retained local history of up to 200 metadata-only events, independently of the compact general team-sharing audit feed.
- Review filtered-ledger exports separately from full-ledger exports to verify the scope of an offline audit handoff.
- Use stable local event IDs when searching, reviewing, copying, or exporting manifest ledger metadata for precise audit references.
- Include a deterministic SHA-256 checksum of the ordered metadata event set in each manifest-ledger Markdown report for local integrity comparison.
- Calculate and copy the same backend event-set checksum for the active manifest ledger view before or after export, without sending audit metadata anywhere.
- Re-verify the latest local SQLite backup on demand before recovery, with maintenance locking and local audit logging.
- View and copy the exact local path of a created or re-verified backup for recovery procedures and offline audit records.
- Review a bounded inventory of the 12 newest local backup snapshots with filename, size, and modification time.
- Re-verify any snapshot listed in the local inventory using an exact CYMOS backup filename; traversal and unknown-file requests are rejected.
- Mark snapshots verified during the current CYMOS session, while keeping durable verification activity in the local audit history.
- Export a metadata-only vault reliability Markdown report with database health and the bounded backup inventory for offline audit or recovery planning.
- Include a deterministic SHA-256 checksum of reliability and snapshot metadata in each local vault reliability report for offline comparison.
- Keep reliability-report checksums path-independent by hashing only the snapshot metadata that is visible in the report.
- Calculate and copy the same path-independent reliability-report checksum for the current local vault state before or after export.
- Return and display the checksum and snapshot count that produced each exported vault reliability report, under one maintenance lock.
- Use collision-resistant reliability-report filenames and browse the 12 newest local reports with metadata and path-copy controls.

## Architecture

```text
Copy / Terminal history / Explicit bookmark / Explicit IDE snippet / Selected local file / Manual note
                |
                v
      Privacy capture guard
                |
                v
         Local SQLite vault
                |
                v
   Deduplication and retention policy
                |
     +----------+----------+
     |                     |
     v                     v
 InsightTrail       Workspace session
     |                     |
     +----------+----------+
                v
 Incident memory and local runbooks
                |
                v
 Workspace replay and Markdown reports
```

All vault, timeline, incident, resolution, report, and backup data remains on the local device unless you deliberately export a file.

## Technology

| Layer | Technology |
| --- | --- |
| Desktop shell | Tauri 2 |
| Backend | Rust |
| Frontend | React 19, TypeScript, Vite |
| Styling | Tailwind CSS |
| Storage | SQLite with SQLx |
| Serialization | Serde |

## Run Locally

### 1. Install prerequisites

- Node.js 20 or newer
- Rust stable toolchain from [rustup](https://rustup.rs/)
- Platform build tools required by Tauri

macOS:

```bash
xcode-select --install
```

On Linux and Windows, install the native dependencies documented by Tauri for your distribution or toolchain before building the desktop application.

### 2. Open the project

```bash
cd /path/to/CtrlCove
```

### 3. Install frontend dependencies

```bash
npm install
```

### 4. Start CYMOS desktop development mode

```bash
npm run tauri dev
```

The first launch compiles Rust, starts Vite on `http://127.0.0.1:1420`, and opens the CYMOS desktop window. Keep the terminal open while developing.

For a browser-only frontend preview without Tauri commands:

```bash
npm run dev
```

The preview is useful for layout work but cannot monitor the operating-system clipboard or call local Tauri commands.

### 5. Install the optional Browser Companion

The desktop vault works without a browser extension. To add opt-in pause guidance for Chrome or Chromium Edge:

```bash
npm run check:extension
npm run test:extension
```

Then open `chrome://extensions` or `edge://extensions`, enable **Developer mode**, select **Load unpacked**, and choose [browser-extension](/Users/manjunathkv/Documents/CtrlCove/browser-extension). Pin **CYMOS Browser Companion**, open the side panel from its toolbar action, and select **Enable this site** on the current HTTP or HTTPS page.

The extension creates a local guide from sanitized page context. Visual screenshots and remote multimodal AI are both disabled by default. To use a compatible vision endpoint, explicitly enable visual context for that site, approve the exact endpoint host-and-port permission, confirm external image sharing, and enter a session-only API key. CYMOS strips invisible display-control characters, rejects malformed or oversized page context before it enters the capture workflow, canonicalizes website-provided autocomplete hints to recognized HTML tokens before any guide, clipboard, or optional remote context uses them, avoids repeat visual-image egress for the same page target for 30 seconds while its service worker is active, opens a one-minute no-screenshot circuit breaker after three consecutive visual-AI failures, validates a new session key before endpoint permission is requested, stores it only while visual AI is enabled, suppresses visual capture before screenshot creation when the focused control appears to collect sensitive data or carries password, OTP, payment, WebAuthn, contact, identity, or address autocomplete metadata, rechecks the exact Chrome endpoint permission immediately before a visual capture, confirms the original tab and document are still active before and after the window-scoped screenshot call, independently sanitizes the text context once more immediately before the remote request, excludes its internal document identifier, treats page text and screenshots as untrusted reference material rather than instructions, limits remote images to supported JPEG/PNG/WebP data URLs within a 2 MiB raw-image budget, limits the complete serialized remote request to 3 MiB, and limits remote JSON responses to 256 KiB. The sensitive-field guard is focused-control based and does not guarantee the rest of a page has no secrets. A tab switch blocks remote image egress, though Chrome does not expose a fully atomic tab-specific screenshot API. Select **Save guide to CYMOS** to place the redacted guide on the local clipboard; the running CYMOS desktop app stores it through the existing clipboard monitor. **Clear AI access** disables remote AI, clears the key, and revokes every managed visual-AI endpoint permission.

The Browser Companion activity panel shows exactly how many session-only guides and metadata events it holds, including whether each guide was local, completed through visual AI, fell back after a visual request, or could not begin visual capture. It stores no screenshots, endpoint URLs, AI responses, or error bodies. **Clear page** removes the current page's capture data; **Clear all** removes every Browser Companion guide and activity record for the current browser session, while keeping site consent, visual-AI settings, and the session-only API key intact. Navigating, closing a tab, selecting **Disable this site**, clearing capture data, or pausing on a newer element cancels pending visual work when possible and always discards its result. A remote abort cannot retract bytes already accepted by a configured endpoint.

## First Operational Workflow

1. Open **Workspace** and create a named workspace.
2. Enter a session title and select **Start capture session**.
3. Copy a command, error, or configuration excerpt, select a local text-based file, or add an InsightTrail note.
4. Review the workspace timeline or use **Memory Replay**.
5. For an open incident, record the command or fix and its outcome in **Incident memory**. Reopen a resolved incident when it regresses and capture the follow-up reason.
6. Search **Local runbook** for a remedy recorded by another workspace.
7. Filter the timeline to a session when investigating a single change window.
8. Select **Export report** to write a local Markdown handoff report. In **Handoff readiness**, enter the recipient and purpose, choose the handling classification and expiry, or reuse an existing entry from the trusted-recipient registry. Confidential handoffs require 1, 7, or 30 day expiry. Optionally uncheck individual timeline events, then approve the recipient locally if it is not already trusted for that handling level. Revoke local trust from the same panel when a recipient should no longer receive handoffs. Run the local safety review after approval. Only a clean scope can create a structured local JSON handoff package signed by the local device key. Use **Verify handoff package** to inspect a received package locally; the metadata-only verification audit updates after each attempt. Expired packages and invalid signed packages cannot be imported; otherwise, explicitly choose **Import as isolated workspace** after verification. Imported handoff workspaces are read-only references and never merge into raw clipboard memory.
9. Review **Local export audit** in the Handoff readiness panel to confirm the scope, declaration, expiry, signer fingerprint, included and excluded metadata counts, local package size, and checksum prefix for each exported handoff.
10. Review **Handoff provenance** in an imported workspace to see its source workspace, scope, declared handling, expiry, signer fingerprint, generation time, local import time, and SHA-256 checksum.
11. In **Local runbook**, select the review status menu to focus on changed SOPs or those due for their 90-day validation. Select the plus icon to save a standalone SOP with tags for later retrieval. Use the download icon to create a local Markdown export and the clock icon to inspect its revision history. Use the check icon to mark the current version reviewed. Restore any prior version from its history with the restore icon; CYMOS records recovery as a new latest revision and requires review again. Edit or delete a standalone SOP with its icon controls; incident-derived remedies remain immutable evidence.
12. Select **End session** when the work is complete.
13. Select **Archive workspace** to preserve a completed project without accepting new captures; restore it later when needed.

### Bounded terminal history import

1. Open **Operations** and select **Bash** or **Zsh**.
2. Select the newest history window to import: 100, 250, 500, or 1,000 commands.
3. Select **Import history**. CYMOS reads only that local history file, filters sensitive and low-signal entries, and reports the import result.

### Explicit terminal command

1. Open **Operations**, choose **Bash** or **Zsh**, and select **Capture command**.
2. Enter the command. Add a host, project, and tags when they provide useful operational context.
3. Select **Save command**. CYMOS rejects known sensitive command patterns, applies local privacy rules, then extracts operational context such as services, hosts, IP addresses, and incident signals.

CYMOS only saves a command you enter in this form. It does not monitor terminal windows, SSH sessions, process output, or shell activity.

### Explicit browser bookmark

1. Open **Memory vault** and select **Browser bookmark**.
2. Enter an HTTP or HTTPS URL, then optionally add a title and comma-separated tags.
3. Select **Save**. The bookmark is validated, checked against local privacy rules, deduplicated, and linked to the active workspace session when one is running.

CYMOS only saves the URL you explicitly provide. It does not inspect browser tabs, history, cookies, page contents, or credentials.

### Explicit IDE snippet

1. Open **Memory vault** and select **IDE snippet**.
2. Paste the code and select its language. Add a project, file path, title, and tags when that context matters.
3. Select **Save snippet**. CYMOS validates the input, checks local privacy rules, and stores it as a code memory linked to the active workspace session when one is running.

CYMOS only saves code that you paste into this form. It does not watch IDE buffers, scan repositories, open files, or collect editor telemetry.

## Verification Checklist

### Clipboard capture

1. Copy `Hello CYMOS`.
2. Open **Memory vault** and confirm the item appears.
3. Search for `CYMOS`.

### Privacy capture guard

1. Open **Platform** and keep protection enabled.
2. Copy a test secret-like value such as `api_key=very-long-test-value-12345`.
3. Confirm it does not appear in the Memory vault.

### Workspace incident flow

1. Start a workspace session.
2. Copy an error such as `nginx: permission denied by SELinux`.
3. Open **Workspace** and review the incident in **Incident memory**.
4. Record `restorecon -Rv /var/www` and the verification outcome.
5. Confirm the incident changes to resolved and the remedy appears in the local runbook.
6. Reopen the incident with a follow-up reason to preserve the remediation and regression history.
7. In **Incident memory**, select a captured, unassigned timeline item and link it to the relevant incident as evidence. Linked evidence remains part of replay, Markdown reports, and JSON handoff packages.

### Explicit local file import

1. Start a workspace capture session.
2. In **Workspace**, choose a text-based log, configuration, script, Markdown file, or other supported document from **Local file import**.
3. Confirm the item appears in the workspace timeline and Memory vault with its local file name as the source.
4. Files larger than 1 MB, unsupported binary files, duplicates, and secret-like text are not stored.

### Reliability and reports

1. Open **Platform** > **Vault reliability**.
2. Run the integrity check, create a verified backup, and use **Verify latest** before relying on a snapshot for recovery.
3. In **Workspace**, select **Export report** and confirm the local Markdown path appears. Select the JSON icon to create a session-scoped handoff package.

### Vault lifecycle policy

1. Open **Platform** > **Vault lifecycle**.
2. Set the retention days, memory limit, and storage budget. Keep **Protect favorites** enabled to preserve pinned knowledge during cleanup.
3. Select **Save lifecycle**, then select **Apply policy** when you are ready to enforce it immediately.
4. Confirm the local result reports removed, retained, and protected memories. The same policy is also evaluated by the local maintenance cycle.

## Development Checks

Frontend build:

```bash
npm run build
```

Rust format and tests:

```bash
cd src-tauri
cargo fmt --check
cargo test
```

Production build:

```bash
npm run build
npm run tauri build
```

## Local Data

CYMOS uses the Tauri application identifier `com.cymos.clipboard`.

Typical app-data locations:

| Platform | Location |
| --- | --- |
| macOS | `~/Library/Application Support/com.cymos.clipboard/` |
| Windows | `%APPDATA%\com.cymos.clipboard\` |
| Linux | `~/.local/share/com.cymos.clipboard/` |

The directory contains the SQLite vault (`cymos.db`), assets, local exports, and verified backups. Manual exports and backups remain until you remove them.

## Project Structure

```text
CtrlCove/
├── src/                     React application and workspace UI
├── src-tauri/
│   └── src/
│       ├── clipboard.rs     Clipboard monitor and classification
│       ├── database.rs      SQLite schema, storage, workspaces, runbooks
│       ├── privacy.rs       Sensitive capture guard
│       ├── insight_trail.rs Timeline and incident models
│       ├── workspace.rs     Workspace, session, resolution models
│       └── main.rs          Tauri command boundary
├── docs/                    Architecture documentation
└── README.md
```

## Design Principles

- Local first and offline by default
- Tauri commands are the application boundary; SQLite repositories own persistence and transactional state changes.
- Transient database contention is retried with bounded exponential backoff; non-transient failures return stable user-safe messages.
- User-owned data
- Explicit capture sources and explicit session boundaries
- Privacy protection before storage
- Verifiable local reliability controls
- Provenance for operational remedies
- No mandatory cloud service

## Near-Term Roadmap

1. Step 01 complete: resilient clipboard capture with canonical deduplication and configurable vault retention.
2. Step 02 complete: explicit, privacy-checked local browser bookmark capture.
3. Step 03 complete: explicit IDE code snippet capture with project-aware local deduplication.
4. Step 04 complete: explicit terminal command capture with safe operational context.
5. Step 05 complete: incident evidence linking backed by the workspace event journal.
6. Step 06 complete: local handoff safety review and export gate for deliberate sharing.
7. Step 07 complete: metadata-only local handoff export audit ledger.
8. Step 08 complete: selective handoff builder with server-validated timeline exclusions and audit counts.
9. Step 09 complete: explicit local handoff declaration with handling classification and preserved provenance.
10. Step 10 complete: local handoff expiry policy with enforced expired-package import blocking.
11. Step 11 complete: device-local Ed25519 handoff signing with signer fingerprint audit and import verification.
12. Step 12 complete: local trusted-recipient registry with classification limits before signed handoff export.
13. Step 13 complete: local trusted-recipient revocation that blocks future handoff exports while preserving audit history.
14. Step 14 complete: metadata-only handoff verification ledger for valid, expired, and rejected package inspections.
15. Step 15 complete: automatic retention for the handoff verification ledger, capped to the latest 200 metadata records.
16. Step 16 complete: trusted-recipient registry picker in the handoff workflow for active and revoked recipients.
17. Step 17 complete: automatic retention for the handoff export ledger, capped to the latest 200 metadata records.
18. Step 18 complete: confidential handoff expiry enforcement; no-expiry confidential exports are blocked.
19. Step 19 complete: confidential handoff signature enforcement; unsigned confidential packages are rejected.
20. Step 20 complete: confidential handoff trusted-signer enforcement before import.
21. Step 21 complete: trusted-signer registry view for active and revoked package signers.
22. Step 22 complete: local team sharing policy controls before synchronization.
23. Step 23 complete: local team sharing readiness preflight with explicit blockers.
24. Step 24 complete: local approved-device registry for team sharing readiness.
25. Step 25 complete: focused local team sharing audit stream.
26. Step 26 complete: local Markdown export for team sharing readiness reports.
27. Step 27 complete: automatic retention for team sharing audit metadata, capped to the latest 200 records.
28. Step 28 complete: local sync dry-run manifest for team sharing readiness.
29. Step 29 complete: local JSON export for team sharing sync dry-run manifests.
30. Step 30 complete: local inspection for exported team sharing dry-run manifests.
31. Step 31 complete: SHA-256 integrity verification for team sharing dry-run manifests.
32. Step 32 complete: device-local Ed25519 signing and signature inspection for team sharing dry-run manifests.
33. Step 33 complete: local trusted-signer gate for signed team sharing dry-run manifest inspection.
34. Step 34 complete: one-click local trust registration for the current CYMOS device signer.
35. Step 35 complete: focused local manifest inspection ledger in the Platform view.
36. Step 36 complete: metadata-only Markdown export for the local manifest inspection ledger.
37. Step 37 complete: local manifest ledger risk summary with verification, warning, and export counts.
38. Step 38 complete: warning-only filter for the local manifest inspection ledger.
39. Step 39 complete: local search for the manifest inspection ledger metadata.
40. Step 40 complete: incremental show-more controls for the local manifest inspection ledger.
41. Step 41 complete: expandable metadata details for individual manifest ledger events.
42. Step 42 complete: one-click metadata copy for individual manifest ledger events.
43. Step 43 complete: metadata-only batch copy for currently visible manifest ledger results.
44. Step 44 complete: one-click reset for local manifest ledger filters, search, paging, and expanded details.
45. Step 45 complete: focused local manifest ledger views for verified inspections and exports.
46. Step 46 complete: metadata-only Markdown export for the currently matching manifest ledger view.
47. Step 47 complete: query-free audit provenance for filtered manifest ledger exports.
48. Step 48 complete: dedicated 200-event local retrieval path for the manifest inspection ledger.
49. Step 49 complete: dedicated local view for query-free filtered manifest ledger exports.
50. Step 50 complete: stable event identifiers across manifest ledger search, details, copy, and export flows.
51. Step 51 complete: deterministic SHA-256 integrity checksum for each manifest ledger report event set.
52. Step 52 complete: live local manifest ledger checksum calculation and copy support.
53. Step 53 complete: on-demand integrity re-verification for the latest local database backup.
54. Step 54 complete: visible and copyable local backup path after creation or verification.
55. Step 55 complete: bounded local backup snapshot inventory in Vault reliability.
56. Step 56 complete: secure on-demand verification for an individual local backup snapshot.
57. Step 57 complete: session-scoped verification status in the local backup inventory.
58. Step 58 complete: metadata-only local vault reliability report export.
59. Step 59 complete: deterministic reliability-report data checksum for offline integrity comparison.
60. Step 60 complete: path-independent reliability-report checksum aligned with visible report metadata.
61. Step 61 complete: live local reliability-report checksum calculation and copy support.
62. Step 62 complete: report-bound reliability checksum metadata returned with each vault export.
63. Step 63 complete: collision-resistant vault report exports and bounded local report inventory.
64. Step 64 complete: opt-in Manifest V3 Browser Companion with local pause guides, transient visual AI, and explicit CYMOS clipboard handoff.
65. Step 65 complete: session-only Browser Companion capture activity trail with visual-AI attempt visibility and no screenshot retention.
66. Step 66 complete: one-click current-page Browser Companion guide and activity clearing with serialized session writes.
67. Step 67 complete: visible Browser Companion session-data inventory, global capture-data clearing, and automatic closed-tab cleanup.
68. Step 68 complete: document-scoped Browser Companion capture state with navigation cleanup and stale visual-result protection.
69. Step 69 complete: consent-revocation cleanup that clears current-page Browser Companion data and invalidates in-flight captures.
70. Step 70 complete: managed visual-AI endpoint permission rotation and one-click AI-access revocation.
71. Step 71 complete: metadata-only visual-AI outcome visibility for local, completed, fallback, and unavailable capture paths.
72. Step 72 complete: latest-capture sequencing and best-effort visual-AI request cancellation across navigation, consent, and clear actions.
73. Step 73 complete: independently sanitized visual-AI request builder with internal lifecycle metadata excluded from remote egress.
74. Step 74 complete: validated visual-image format and 2 MiB remote egress budget with local-only fallback.
75. Step 75 complete: bounded 256 KiB streamed visual-AI JSON response parser with safe local fallback.
76. Step 76 complete: redirect-free, no-credential visual-AI transport with JSON media-type validation.
77. Step 77 complete: schema-validated, 16 KiB-bounded visual-AI guide content with honest fallback outcomes.
78. Step 78 complete: explicit prompt-injection boundary for untrusted webpage and screenshot context.
79. Step 79 complete: pre-capture sensitive-field suppression for visual AI with local-guide continuity.
80. Step 80 complete: centralized endpoint and model configuration validation before visual-AI permission or egress.
81. Step 81 complete: pre-capture recheck of managed and live Chrome endpoint permission state.
82. Step 82 complete: pre/post-capture active-tab and document continuity guard for window-scoped screenshots.
83. Step 83 complete: bounded, header-safe session API-key validation before permission, storage, or egress.
84. Step 84 complete: bounded and canonical browser-context validation before capture processing.
85. Step 85 complete: in-memory same-target visual egress throttle with uninterrupted local guidance.
86. Step 86 complete: invisible-control and bidirectional-text neutralization for browser context and clipboard handoff.
87. Step 87 complete: 3 MiB serialized visual-AI request budget enforced before network egress.
88. Step 88 complete: metadata-only visual-AI circuit breaker with local-guide continuity during endpoint cooldown.
89. Step 89 complete: HTML autocomplete-aware sensitive-field guard before visual capture.
90. Step 90 complete: monotonic AI-configuration epoch that discards stale visual work and responses before they are stored or shown.
91. Step 91 complete: conservative sensitive-flow URL-path guard before visual screenshot capture.
92. Step 92 complete: request-only visual-AI timeout that preserves cancellation semantics and endpoint circuit health.
93. Step 93 complete: serialized session-only per-origin visual-capture privacy budget with local fallback.
94. Step 94 complete: live current-site visual-capture allowance visibility in the Browser Companion panel.
95. Step 95 complete: next-allowance time for exhausted visual-capture budgets.
96. Step 96 complete: deterministic 100-origin cap for session visual-capture budget metadata.
97. Step 97 complete: visible visual-AI circuit-breaker cooldown with automatic retry-status refresh.
98. Step 98 complete: explicit restrictive Manifest V3 CSP for Browser Companion pages and worker.
99. Step 99 complete: contact, identity, and address autocomplete privacy guard before visual capture.
100. Step 100 complete: canonical autocomplete metadata before guide storage and optional visual-AI egress.
43. Optional team sharing and synchronization, kept separate from the local vault by default.

## License

MIT License.
