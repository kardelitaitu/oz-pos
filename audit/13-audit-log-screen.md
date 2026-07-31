# Audit Log Screen Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** AuditLogScreen — append-only event visibility, filters, pagination, review workflow, authorization, privacy, localization, and tests  
> **Status:** AUDITED · authorization and compliance-readiness findings require remediation  
> **Production code changed:** None

## Scope

This audit evaluates AuditLogScreen against the universal checklist in `audit/AUDIT_JULY_2026.md`: event integrity and functionality, loading/error/retry states, pagination and filtering, accessibility and localization, theming, performance, security and privacy, and quality assurance.

Inspected areas:

- `ui/src/features/audit/AuditLogScreen.tsx`
- `ui/src/features/audit/AuditLogScreen.css`
- `ui/src/api/audit.ts`
- `apps/desktop-client/src/commands/audit.rs`
- `crates/oz-core/src/db/audit.rs`
- Audit-log schema migrations and command registration
- `ui/src/__tests__/AuditLogScreen.test.tsx`
- `ui/src/locales/shared.ftl` and `shared.id.ftl`

## Architecture summary

The screen requests 50 audit entries at a time through `listAuditLog(limit, offset)`. The backend reads the global SQLite audit table in reverse chronological order using `LIMIT/OFFSET`. The renderer performs outcome and free-text filtering over only the entries currently loaded, appends additional pages through Load More, and displays a locally stored “last reviewed” timestamp. Manager users can mark the current review time in browser `localStorage`; no backend review record or export command is exposed by this screen.

Audit entries are append-only at the core database helper and contain user, action, target, details, outcome, and timestamp fields. The desktop `list_audit_log` command currently opens the global database and does not resolve a session-bound store or enforce a permission in the inspected handler. React renders details as text, so stored markup is not directly interpreted as HTML in this component, but the database accepts arbitrary detail strings without a sensitive-field policy.

## Findings

### AUD-01 — Audit-log IPC has no backend authorization or tenant scope (P0/P1 conditional security risk)

**Evidence:** `apps/desktop-client/src/commands/audit.rs::list_audit_log` locks `state.db`, creates a `Store`, and returns `store.list_audit_entries(...)`. It does not accept a session token, resolve a store, or call the permission gate. The UI exposes manager-oriented review controls through `isManager`, but that is not an IPC authorization boundary. The core query reads the global `audit_log` table.

**Impact:** If the command is reachable by a lower-privileged caller or in a multi-store deployment, an invocation can disclose audit events outside the active store and bypass the UI's manager restriction. The precise exploitability depends on the desktop command exposure and deployment wiring, so the severity is conditional but the missing backend boundary is confirmed.

**Recommendation:** Add `list_audit_log_scoped(session_token, args)` that resolves the store and authenticated user server-side, enforces an audit-view permission, and applies a store/tenant predicate. Remove caller-controlled identity from the authorization path, deprecate the global command, and add IPC tests for unauthenticated, cross-store, and insufficient-role calls.

**Status:** Open · P0/P1 conditional

### AUD-02 — Filtering and unreviewed counts cover only the loaded page set

**Evidence:** `filteredEntries` applies `outcomeFilter` and `searchQuery` to the local `entries` array. `countUnreviewed(entries, lastReviewed)` also counts only that array. The initial request is limited to 50 rows, and there is no filter or count parameter in `listAuditLog`.

**Impact:** A failure, critical action, or unreviewed event outside the loaded 50 rows is invisible to the active filter and absent from the “new” badge until the operator manually loads more. A manager can therefore see zero matching results or an understated review count while older loaded pages contain relevant events.

**Recommendation:** Move filtering and review-count computation to scoped backend queries with explicit filter parameters and total counts, or clearly label the current values as “loaded entries.” Add server-side pagination metadata and tests proving matches beyond the first page are discoverable.

**Status:** Open · P1 compliance/UX

### AUD-03 — Offset pagination can duplicate or skip entries while new events arrive

**Evidence:** The backend orders by `created_at DESC` and applies `OFFSET`. The UI requests `offset + limit` when Load More is clicked and appends the returned page. New audit rows inserted between page requests shift the offset boundary, so the second page is not a stable continuation of the first. The query has no unique timestamp/ID tie-breaker or cursor.

**Impact:** During active operation, an auditor can see duplicate rows or permanently miss rows while paging. This weakens the completeness of an audit review and makes the displayed count unreliable.

**Recommendation:** Use a stable cursor such as `(created_at, id)` with deterministic ordering, return a continuation cursor and `has_more`, and deduplicate by entry ID in the UI as a defense in depth. Add an integration test that inserts an event between page requests.

**Status:** Open · P1 integrity risk

### AUD-04 — “Mark Reviewed” is device-local and is not an auditable review event

**Evidence:** `handleMarkReviewed()` writes `audit-last-reviewed` only to browser `localStorage`. The value is not stored in the database, associated with a user/store, or represented as an audit event. Clearing browser data or using another terminal resets the state.

**Impact:** Review status is not shared across managers, cannot be verified centrally, and cannot support compliance evidence. The badge can also be marked reviewed without proving that all matching events were loaded, because the count itself is page-local.

**Recommendation:** Persist review checkpoints server-side with tenant, reviewer, timestamp, and cursor/high-water mark. Write a corresponding audit event, enforce manager permission on the mutation, and expose the review history. Keep local storage only as an optional UI cache.

**Status:** Open · P1 compliance gap

### AUD-05 — Refresh and Load More requests lack request-generation protection

**Evidence:** `cancelledRef` is set only by the effect cleanup on unmount. It does not distinguish overlapping `load(0)` calls from Refresh, Retry, or an in-flight Load More request. A request can resolve after another request and write `entries`, `offset`, `hasMore`, `loading`, or append data based on stale state.

**Impact:** Rapid refresh/load-more interactions can replace fresh data with an older page or append pages in the wrong order. The button is disabled while `loading`, which reduces but does not eliminate overlap across state transitions and external triggers.

**Recommendation:** Use a monotonically increasing request ID or AbortController, treat refresh as a new generation that invalidates append requests, and deduplicate appended rows by ID. Add deferred-promise tests for out-of-order refresh and pagination responses.

**Status:** Open · P2

### AUD-06 — Audit details have no sensitive-data minimization or structured policy

**Evidence:** `Store::log_audit` persists arbitrary `details` strings without field filtering or size policy, and the schema/core tests explicitly preserve HTML strings. The screen truncates details to 60 characters but does not redact secrets or PII. React text rendering prevents the displayed string from being interpreted as markup in this component, so this is not a confirmed XSS execution path.

**Impact:** Tokens, PIN-related material, customer data, or other secrets written by upstream callers can remain in the audit database and be exposed to every authorized log viewer. Large arbitrary details also increase storage and rendering costs.

**Recommendation:** Define an allowlisted structured-details schema per action, redact credentials and secrets before persistence, cap payload size, and separate privileged detail access from ordinary log browsing. Add tests asserting sensitive keys are removed and oversized payloads are rejected or summarized.

**Status:** Open · P1/P2 privacy risk

### AUD-07 — Date formatting ignores the application locale

**Evidence:** `formatDate()` calls `toLocaleDateString(undefined, ...)`, and the reviewed timestamp uses `new Date(lastReviewed).toLocaleDateString()` without the selected locale. No locale is derived from the Fluent localization context.

**Impact:** Date order, month names, and time formatting can differ from the selected application language and from other screens. This is especially confusing for compliance records where exact chronology matters.

**Recommendation:** Pass the active locale to `Intl.DateTimeFormat`, preserve a timezone policy, and include an explicit accessible ISO/UTC value where needed. Add English/Indonesian formatting tests.

**Status:** Open · P2 i18n

### AUD-08 — Action and outcome presentation has localization drift and raw machine values

**Evidence:** `ACTION_FLUENT_IDS` covers a fixed subset of action strings, while unknown actions render the raw key. The outcome badge renders raw `entry.outcome` values. The inspected core tests use actions such as `sale.create`, `user.login`, and `bulk.import`, which are not all mapped by the screen's action map. Several user-visible fallback strings and count text are embedded in JSX.

**Impact:** New or legacy events appear as machine identifiers or English fallback text, reducing comprehension for localized operators and making critical events harder to recognize consistently.

**Recommendation:** Define a centralized action/outcome catalog with localized labels and a safe localized fallback for unknown keys. Keep the raw action available as secondary technical metadata. Add bundle-parity tests against known emitted actions and both locale bundles.

**Status:** Open · P2 i18n/UX

### AUD-09 — Audit screen has no export or immutable review handoff

**Evidence:** `ui/src/api/audit.ts` exposes only `listAuditLog`; no export API or UI action exists in the inspected screen. Although shared locale bundles contain generic audit export action labels, the audit screen does not provide an export flow or an evidence snapshot.

**Impact:** Managers cannot produce a bounded, filtered audit extract for incident response, external review, or retention workflows. They must rely on the incomplete page-local view or external copying.

**Recommendation:** Add a permissioned, server-side export that records filter scope, cursor/date range, requesting user, and export event. Prefer a streamed CSV/JSON/PDF artifact with redaction and a deterministic snapshot boundary rather than exporting the mutable page state.

**Status:** Open · P2 capability/compliance

### AUD-10 — Critical-row and detail styling contains token fallback and inline-style drift

**Evidence:** `AuditLogScreen.css` uses hardcoded fallback colours in `var(--color-success-bg, #f0fdf4)`, `var(--color-danger, #dc2626)`, and related declarations. The component also uses inline styles for table indicator width/padding and icon spacing. The project’s theme-token convention favors centralized semantic tokens.

**Impact:** Theme and high-contrast behavior can diverge when tokens are missing, and styling compliance becomes harder to test. The fixed fallback colours are particularly fragile across dark themes.

**Recommendation:** Replace fixed colour fallbacks with guaranteed semantic tokens or theme-owned fallback mappings, move layout declarations to CSS classes, and add the screen to theme-token compliance checks.

**Status:** Open · P3 theming/maintenance

### AUD-11 — Focused tests omit security, completeness, and review semantics

**Evidence:** `AuditLogScreen.test.tsx` has 20 passing tests covering rendering, loading, retry, table output, known/unknown actions, truncation, local filters, and offset invocation. It does not cover backend authorization/scoping contracts, events beyond the first page, offset shifts, out-of-order responses, local-review persistence semantics, locale date formatting, sensitive detail handling, or export permissions.

**Impact:** Important compliance and privacy regressions can pass the current UI suite.

**Recommendation:** Add backend integration tests for scope/permission and query bounds, UI tests for page-boundary filtering and review checkpoints, race tests with deferred promises, locale tests, and privacy/redaction tests.

**Status:** Open · P3 QA gap

## Positive controls observed

- Audit events are persisted through bound SQL parameters in an append-only helper.
- The screen has loading skeleton, initial error/retry, successful-empty, and filtered-empty states.
- Search and outcome filters are keyboard-accessible native controls with radio semantics.
- Critical actions and failed outcomes receive visual row emphasis.
- React renders details as text rather than injecting HTML.
- Focused UI tests cover 20 important rendering and interaction paths.
- The audit query supports bounded `LIMIT/OFFSET` pagination rather than loading the entire table at once.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/AuditLogScreen.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **20 passed, 0 failed**
- TypeScript typecheck: **passed with 0 errors**
- Report existence and Markdown trailing-whitespace validation: **passed after report generation**
- No dedicated Rust audit-command test count is claimed; backend source and existing core audit tests were inspected but not run as a focused command during this audit

## Recommended remediation order

1. **AUD-01:** Add session-scoped backend authorization and tenant filtering.
2. **AUD-02 through AUD-04:** Make pagination, counts, and review checkpoints complete and auditable.
3. **AUD-05 and AUD-06:** Protect request generations and minimize/redact sensitive details.
4. **AUD-07 through AUD-09:** Repair locale/action presentation and add permissioned export.
5. **AUD-10 and AUD-11:** Complete theme-token cleanup and security/compliance test coverage.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
