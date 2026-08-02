# Error-Handling Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Error boundaries, async failures, toast consistency, retry behavior, loading/error fallbacks, cancellation, and logging  
> **Status:** ✅ **FULLY REMEDIATED** — all 10 findings (ERR-01→ERR-10) closed across 5 commits  
> **Remediation commits:** `10f1bae0` (ERR-01/05/06), `c586c3d6` (ERR-04/05), `537f5867` (ERR-07/08), `31adb7c3` (ERR-02/03), `5dacd75f` (ERR-09/10)

## Scope

This audit evaluates the error-handling surface against the universal checklist in `audit/AUDIT_JULY_2026.md`: render-error containment, async failure recovery, error-to-user mapping, retry behavior, loading and error-state transitions, cancellation and stale updates, toast semantics, logging, backend error contracts, security-sensitive error disclosure, and test coverage.

Inspected areas:

- `ui/src/components/ErrorBoundary.tsx`
- `ui/src/components/ErrorState.tsx`
- `ui/src/frontend/shared/ErrorState.tsx`
- `ui/src/frontend/shared/Toast.tsx`
- `ui/src/contexts/AppProviders.tsx`
- `ui/src/contexts/WorkspaceContext.tsx`
- `ui/src/components/ConnectionStatus.tsx`
- `ui/src/features/offline/OfflineQueueScreen.tsx`
- `ui/src/features/audit/AuditLogScreen.tsx`
- Representative category, currency, loyalty, promotion, gift-card, inventory, and purchasing screens
- `ui/src/utils/logged-invoke.ts`
- `apps/desktop-client/src/error.rs`
- `crates/oz-core/src/error.rs`
- Existing error, toast, API-contract, and screen tests

## Architecture summary

The desktop application wraps its provider tree in a root `ErrorBoundary` through `AppProviders`. The boundary catches synchronous render and lifecycle errors and exposes a local “Try Again” reset. Feature screens generally handle asynchronous failures themselves with one of three patterns: local error state plus a retry button, an error toast, or a silent catch that preserves the existing state.

The canonical toast implementation is `ui/src/frontend/shared/Toast.tsx`, backed by `useAnimatedToastQueue`. It provides per-item dismissal, auto-dismiss durations, exit animation state, assertive live regions, and a race-safe clear operation. A legacy `ui/src/hooks/useToast.tsx` implementation and its corresponding test still exist, but the application provider imports the canonical `@/frontend/shared/Toast` path; this creates a maintenance and contract-drift risk.

Rust commands return a typed `AppError` with `kind`, optional typed `subKind`, and a message. The UI’s `loggedInvoke` wrapper measures and logs command timing in development, but it deliberately rethrows the original value and does not normalize the typed error into a shared front-end error model.

## Findings

### ERR-01 — Root render errors are contained, but asynchronous errors remain outside the boundary

**Evidence:** `ui/src/components/ErrorBoundary.tsx` implements `getDerivedStateFromError` and `componentDidCatch`, and `ui/src/contexts/AppProviders.tsx` places it around the provider tree. The existing `ErrorBoundary.test.tsx` explicitly documents that errors thrown asynchronously from `useEffect` are not caught by a class error boundary.

**Impact:** A rejected promise, event-handler exception, timer callback failure, or background task can still surface as an unhandled browser/runtime error without a consistent recovery UI. This is expected React behavior, but the application has no equivalent global async failure surface.

**Recommendation:** Keep the boundary for render failures, and add a small global failure-reporting layer for `window.error` and `unhandledrejection` that logs a redacted diagnostic and presents a recoverable notification. Do not treat every rejected promise as fatal; define an explicit classification for expected API failures versus unexpected defects.

**Severity:** P1 · reliability

**Status:** Remediated — commit `10f1bae0` (ERR-01/05/06: typed AppError normalizer + global async-failure reporter)

### ERR-02 — Error-boundary fallback is not integrated with the active locale or design-token system

**Evidence:** `ErrorBoundary.tsx` constructs a module-level English `FluentBundle` containing “Something went wrong” and “Try Again” because the class component cannot use hooks. Its fallback also uses inline `sans-serif`, hardcoded `#ef4444` and `#737373`, and inline layout styles. The normal application uses `LocaleProvider` and CSS theme tokens.

**Impact:** A catastrophic render failure can show English-only copy and colors that do not follow the selected locale, brand, dark theme, forced-colors mode, or future design-system changes. The fallback is the screen users see when the rest of the UI is least reliable, so it needs especially conservative styling and localization behavior.

**Recommendation:** Move fallback layout and colors to a token-backed CSS class. Use a locale-independent emergency fallback only when localization itself is unavailable, but otherwise inject a localized formatter/provider or pass translated strings from a wrapper. Add tests for fallback semantics and token usage rather than snapshotting inline styles.

**Severity:** P1 · UX and accessibility

**Status:** Remediated — commit `31adb7c3` (ERR-02/03: tokenized/localized ErrorBoundary + consolidated primitives)

### ERR-03 — Two ErrorState implementations can drift in behavior and styling

**Evidence:** Both `ui/src/components/ErrorState.tsx` and `ui/src/frontend/shared/ErrorState.tsx` export an `ErrorState` component with nearly identical props and markup. One imports `Button` from `./Button`; the other imports `@/components/Button`. The application contains separate component and frontend/shared design-system trees.

**Impact:** New screens can import different versions accidentally, producing inconsistent retry buttons, styling, localization behavior, and future fixes. A reliability primitive should have one source of truth so that accessibility or retry-state fixes reach every consumer.

**Recommendation:** Select one canonical ErrorState module, re-export it from the compatibility path if needed, and add an import-policy or compile-time test preventing a second implementation. Apply the same consolidation review to EmptyState and Spinner where duplicate paths exist.

**Severity:** P2 · consistency and maintainability

**Status:** Remediated — commit `31adb7c3` (ERR-02/03: shared ErrorState/EmptyState/Spinner are thin re-exports + import-policy test)

### ERR-04 — Async failure presentation is inconsistent across feature screens

**Evidence:** `AuditLogScreen`, `ExchangeRateScreen`, `OfflineQueueScreen`, and `TerminalManagementScreen` keep an error message and render a retryable error view. `PromotionManagementScreen`, `TransitAuditScreen`, `GiftCardsScreen`, and `ThresholdConfigScreen` use error toasts. `CategoryManagementScreen` catches initial-load errors with only a comment (“IPC unavailable”), while `LoyaltyManagementScreen` catches initial-load errors and keeps the prior state without setting an error or toast.

**Impact:** Users receive materially different recovery affordances for comparable failures. Some failures are visible and retryable, some are transient notifications, and some are silent. Silent initial-load failures can look like a legitimate empty dataset and lead staff to make decisions using incomplete information.

**Recommendation:** Define a screen-level policy: initial-load failures must render an error state with retry; mutation failures may use a toast plus preserved form state; background refresh failures may use a non-blocking status indicator. Replace silent catches with an observable state and add a shared async-state helper or hook.

**Severity:** P1 · user trust and data correctness

**Status:** Remediated — commit `c586c3d6` (ERR-04/05: ~50 screens routed through user-safe error mapper)

### ERR-05 — Several screens expose raw backend error messages directly to users

**Evidence:** `AuditLogScreen` assigns `err.message` directly to rendered error content. `LoyaltyManagementScreen`, `GiftCardPayment`, `IssueGiftCardModal`, `CustomerManagementScreen`, `CategoryManagementScreen`, `PromotionManagementScreen`, and other feature flows use `err instanceof Error ? err.message : ...` as the displayed message. Rust `AppError` messages include database, validation, and internal details.

**Impact:** When a backend failure reaches a screen as an `Error` and that screen renders `err.message`, internal implementation details, SQL/database wording, identifiers, or infrastructure information can leak into the UI. Other serialized Tauri error shapes may instead take a generic fallback path, but neither behavior is a shared, localized policy. Raw messages may be too technical for a cashier, and inconsistent messages make support and telemetry aggregation difficult.

**Recommendation:** Add a shared front-end error mapper keyed by the typed `AppError.kind`/`subKind` contract. Return localized, user-safe messages for expected validation, permission, session, conflict, offline, and hardware failures; retain the raw error only in redacted development diagnostics. Add tests ensuring internal messages never become the default user-facing copy.

**Severity:** P1 · security, localization, and UX

**Status:** Remediated — commits `10f1bae0` + `c586c3d6` (ERR-05 sweep + `5dacd75f` policy gate caught 3 chart-widget leaks)

### ERR-06 — `loggedInvoke` logs timing but does not normalize, classify, or correlate failures

**Evidence:** `ui/src/utils/logged-invoke.ts` logs command start/success/failure and rethrows the original error. It has no request correlation ID, error-kind extraction, redaction, retry classification, or production telemetry hook. `apps/desktop-client/src/error.rs` provides typed serialization, but there is no shared adapter at the UI boundary.

**Impact:** Every caller must independently decide whether an error is retryable, permission-related, stale-session-related, or user input. This duplicates logic and causes the inconsistent behavior documented in ERR-04/ERR-05. Timing logs cannot be connected reliably across a user action and its retry.

**Recommendation:** Introduce a typed `AppError` parser/normalizer at the IPC client boundary. Preserve the original cause for development diagnostics, attach a correlation ID, classify retryability, and expose a redacted structured event hook. Keep the default production log payload free of tokens, customer data, and raw SQL details.

**Severity:** P2 · observability and consistency

**Status:** Remediated — commit `10f1bae0` (loggedInvoke normalizes via parseAppError + structured IPC error event)

### ERR-07 — Offline queue polling ignores failures and has no explicit unmount guard for its async callback

**Evidence:** `ui/src/features/offline/OfflineQueueScreen.tsx` starts a ten-second `setInterval` that awaits `pendingOfflineCount()` and `getOfflineQueueStatusSummary()`. Its catch block intentionally does nothing. The interval is cleared on unmount, but an already-running promise can still resolve after unmount and call `setPendingCount` or `setConflictCount`; the interval can also begin another poll while an earlier request is still pending.

**Impact:** A polling outage is invisible to the user, and stale results can update state after navigation. The queue may display an old pending/conflict count while the primary list is already stale or failed. Overlapping requests add unnecessary load and make ordering nondeterministic. This is especially confusing in the offline workflow, where status freshness is central to the feature.

**Recommendation:** Track a mounted/request generation guard and ignore late results. Surface the last refresh time and a non-blocking stale indicator after repeated poll failures. Use a recursive timeout or abortable request rather than overlapping intervals when a poll can run longer than its period.

**Severity:** P1 · offline correctness

**Status:** Remediated — commit `537f5867` (ERR-07: generation-safe poll + stale indicator)

### ERR-08 — Connection-status checks can overlap and retry scheduling is not request-aware

**Evidence:** `ui/src/components/ConnectionStatus.tsx` starts a check on mount, schedules another check after completion, and invokes `runCheck()` immediately on every `online` event. It uses a local `AbortController` for a five-second request timeout, but there is no in-flight guard or controller ref to cancel a previous check before starting another one. The component uses exponential backoff after failures and jittered intervals after success.

**Impact:** Rapid online/offline transitions or repeated online events can create concurrent health requests and multiple scheduling paths. A slower earlier response can update the indicator after a newer check has begun, making status and latency briefly misleading.

**Recommendation:** Store the active controller and check generation in refs, abort or supersede the previous request before a new check, and ensure only the latest generation schedules the next timeout. Add tests for repeated online events, timeout cleanup, and stale responses.

**Severity:** P2 · network-state correctness

**Status:** Remediated — commit `537f5867` (ERR-08: in-flight guard + request-aware scheduling)

### ERR-09 — Retry controls do not consistently preserve or reset state intentionally

**Evidence:** `OfflineQueueScreen.handleSyncAll` sets `syncing`, calls `retryOfflineSync`, then calls `load`; `load` clears the page error and toggles the main loading state. `AuditLogScreen.load` clears errors and replaces or appends entries based on the call. Other screens reload after mutations, while several mutation handlers preserve form state only incidentally. There is no shared retry contract specifying whether stale data remains visible during refresh.

**Impact:** The inspected retry flows can clear their current error and toggle loading while reloading, while other flows preserve prior rows and show only a toast. Across the feature set this creates a risk of blank/skeleton transitions or stale rows without an explicit “refreshing” indicator, and a retry may discard useful data or filters differently from another screen. Operational screens benefit from showing known data while retrying rather than implying that data disappeared.

**Recommendation:** Standardize async state as `idle/loading/refreshing/success/error`, preserve existing data during refresh, and expose retry intent in accessible status text. Document whether retries reset pagination and filters. Add tests for retry after initial failure and retry while stale data is visible.

**Severity:** P2 · recovery UX

**Status:** Remediated — commit `5dacd75f` (ERR-09: shared retry-state contract + refreshing status)

### ERR-10 — Error and toast test coverage is strong for primitives but thin for application-wide failure paths

**Evidence:** `ErrorBoundary.test.tsx`, `ErrorState.test.tsx`, `Toast.test.tsx`, and API contract tests cover primitive behavior, error propagation, retry buttons, animation, and typed command invocation. Screen tests exist for selected flows such as `AuditLogScreen`, but there is no discovered global `unhandledrejection`/`window.error` recovery test, shared error-mapper contract test, or cross-screen policy test that detects silent catches and raw error rendering.

**Impact:** Primitive regressions are likely to be caught, but architectural drift can continue: a new screen can silently swallow load errors or show a raw backend message while all existing primitive tests pass.

**Recommendation:** Add a focused error-policy test suite covering the normalized AppError mapper, redaction, retry classification, global failure reporting, and representative initial-load/mutation/background-refresh patterns. Add a static check or review checklist for empty catches in feature code.

**Severity:** P2 · quality assurance

**Status:** Remediated — commit `5dacd75f` (ERR-10: errorPolicyCompliance static gate + contract tests)

## Positive controls observed

- A root React error boundary is present and has a user-triggered reset path.
- The boundary and ErrorState fallbacks use alert semantics, and toast dismiss controls have localized accessible names.
- The canonical toast queue handles per-item auto-dismiss, manual dismissal, exit animation, persistent toasts, and race-safe collective clearing.
- Representative data screens provide loading skeletons, empty states, and retry buttons.
- Audit-log loading uses a cancellation ref to avoid setting state after its effect has been cleaned up.
- Workspace loading uses cancellation flags around asynchronous workspace and screen discovery operations.
- Connection checks use `AbortController` and exponential backoff rather than an unbounded rapid retry loop.
- Rust `AppError` is discriminated and serializable, with typed core and hardware sub-kinds.
- API contract tests verify that backend errors are propagated rather than silently converted into success.

## Test and validation results

Focused validation completed for this report:

```text
cd ui
npx vitest run src/__tests__/ErrorBoundary.test.tsx src/__tests__/ErrorState.test.tsx src/__tests__/Toast.test.tsx src/__tests__/ConnectionStatus.test.tsx src/__tests__/OfflineQueueScreen.test.tsx src/__tests__/AuditLogScreen.test.tsx
npm run typecheck
```

Results:

- Report existence and Markdown formatting: **passed**; no non-hard-break trailing whitespace
- Focused error-handling tests: **passed**; 6 files, 75 tests
- UI TypeScript typecheck: **passed** with 0 errors
- No production code was changed during this audit

These checks establish existing control behavior and representative retry/error coverage; they do not prove that every feature follows the same error policy.

## Recommended remediation order

1. **ERR-01/ERR-05/ERR-06:** Add typed, localized, redacted error normalization and a deliberate global async-failure policy. → `10f1bae0`
2. **ERR-04:** Eliminate silent catches and standardize initial-load, mutation, and background-refresh recovery states. → `c586c3d6`
3. **ERR-07/ERR-08:** Make polling and connectivity checks generation-safe and expose stale-status information. → `537f5867`
4. **ERR-02/ERR-03:** Tokenize/localize the emergency boundary and consolidate duplicate shared error primitives. → `31adb7c3`
5. **ERR-09/ERR-10:** Define a shared retry state contract and add cross-screen policy tests/static checks. → `5dacd75f`

## Audit status

✅ **FULLY REMEDIATED (2026-08-02).** All 10 findings are closed by five remediation commits:

| Finding | Commit | Validation |
|---|---|---|
| ERR-01/05/06 — typed normalizer + global async reporter + loggedInvoke normalization | `10f1bae0` | typecheck ✓ · app-error 24 tests ✓ · GlobalErrorReporter ✓ · lint+i18n ✓ |
| ERR-04/05 — ~50 screens routed through user-safe mapper | `c586c3d6` | typecheck ✓ · 274 tests ✓ · lint+i18n ✓ |
| ERR-07/08 — generation-safe polling + in-flight guard | `537f5867` | typecheck ✓ · 23 tests ✓ · lint+i18n ✓ |
| ERR-02/03 — tokenized ErrorBoundary + consolidated primitives | `31adb7c3` | typecheck ✓ · 49 tests ✓ · lint+i18n ✓ |
| ERR-09/10 — retry-state contract + policy gate | `5dacd75f` | typecheck ✓ · 87 tests ✓ · lint+i18n ✓ |

The `errorPolicyCompliance` static gate (`5dacd75f`) immediately caught three real ERR-05 leaks in the reporting widgets (CategoryPieChartWidget, HourlyHeatmapWidget, RevenueLineChartWidget), which were fixed in the same commit — the drift guard is already earning its keep.
