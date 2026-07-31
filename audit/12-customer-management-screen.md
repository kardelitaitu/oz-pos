# Customer Management Screen Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** CustomerManagementScreen — customer CRUD, search, privacy-sensitive fields, sales/loyalty relationships, localization, and tests  
> **Status:** AUDITED · authorization, data-integrity, and UX findings require remediation  
> **Production code changed:** None

## Scope

This audit evaluates CustomerManagementScreen against the universal checklist in `audit/AUDIT_JULY_2026.md`: functionality and state management, loading/error/empty states, accessibility and localization, theming, performance, security and authorization, data integrity, privacy, and quality assurance.

Inspected areas:

- `ui/src/features/customers/CustomerManagementScreen.tsx`
- `ui/src/features/customers/CustomerManagementScreen.css`
- `ui/src/api/customers.ts`
- `apps/desktop-client/src/commands/customers.rs`
- `crates/oz-core/src/db/customers.rs`
- Customer, sales, and loyalty schema migrations
- `ui/src/frontend/shared/SettingsPopup.tsx`
- `ui/src/hooks/useFocusTrap.ts`
- `ui/src/__tests__/CustomerManagementScreen.test.tsx`
- `ui/src/locales/customers.ftl` and `customers.id.ftl`

## Architecture summary

`CustomerManagementScreen` loads customers through `listCustomersScoped(sessionToken)`, filters the in-memory list by name, email, and phone, and renders a searchable table. It provides create and edit forms in the shared `SettingsPopup`, and the delete action currently invokes the delete command immediately from the row button. The screen has a loading skeleton, successful-empty state, and search no-match state.

The API exposes a scoped list command, but create/update/delete remain legacy helpers carrying a caller-supplied `userId` and invoking unscoped commands. The desktop handlers validate name, email, and phone and check permissions against that supplied user ID, but they open the global database rather than resolving a session-bound store. Customers are referenced by sales and loyalty-account schema, while this screen has no customer history view or customer-history API call.

The shared `SettingsPopup` provides dialog semantics, focus trapping, Escape handling, backdrop dismissal, and body scroll locking. The customer screen still contains hardcoded labels/placeholders, empty localization attributes, swallowed load/delete errors, and action controls that are not sized for the project's touch-target convention.

## Findings

### CUST-01 — Customer mutations are unscoped and trust a caller-supplied user ID (P1 tenant and authorization risk)

**Evidence:** The screen calls `createCustomer`, `updateCustomer`, and `deleteCustomer`, whose API arguments include `userId` and invoke `create_customer`, `update_customer`, and `delete_customer` without a session token. The desktop handlers open `state.db` directly. Permission checks use `require_permission_for_user(&store, &args.user_id, ...)`, so the authorization identity is supplied by the caller rather than resolved from an opaque session. Only `listCustomersScoped(sessionToken)` uses the store resolved from the current session.

**Impact:** In a multi-store deployment, customer mutations can target the global database instead of the active store. A caller able to invoke the IPC command may also select the user identity used for the permission check. The exact exposure depends on deployment and command access, but this is not a valid session-bound authorization boundary.

**Recommendation:** Add session-scoped create/update/delete commands. Resolve both store and authenticated user from the session token, enforce the customer permission for that session user, and remove `userId` from frontend mutation arguments. Deprecate the global mutation commands and add IPC tests proving the session token is required and cannot be substituted with another user ID.

**Status:** Open · P1

### CUST-02 — Delete is immediate and has no confirmation or relationship-aware preview

**Evidence:** The row delete button calls `confirmDelete(customer.id)` directly. `confirmDelete()` immediately invokes `deleteCustomer({ userId, id })`; no confirmation dialog or second explicit confirmation state is present in `CustomerManagementScreen.tsx`. The database schema references customers from sales and loyalty accounts, but the UI does not show affected records or explain what deletion means for those relationships.

**Impact:** An accidental click can attempt a destructive customer mutation. Depending on foreign-key enforcement and the existing customer relationships, deletion may be rejected or may have downstream effects. The operator receives no opportunity to review the customer's identity, sales history, or loyalty relationship before confirming.

**Recommendation:** Add a localized `alertdialog` confirmation with customer name, relationship warning, Cancel/Delete actions, focus trapping, and Escape handling. Prefer an explicit archive/deactivate policy for customers with sales or loyalty history; otherwise return an affected-record result and make the relationship behavior transactional and visible.

**Status:** Open · P1 data-integrity/UX risk

### CUST-03 — Load failures appear as an empty customer list

**Evidence:** `load()` catches every `listCustomersScoped` error with `// IPC unavailable` and does not set an error state. Its `finally` sets `loading` to false. If the initial list remains empty, the component renders the successful “No customers yet” state and add CTA.

**Impact:** Authentication expiry, database failure, permission denial, and a genuinely empty customer database are indistinguishable. Operators may create duplicate records or conclude that customer data has disappeared. There is no retry action.

**Recommendation:** Track `loadError` separately from the customer collection, render a localized error/retry state, and preserve the last successful list during refreshes. Add tests for initial failure and retry recovery.

**Status:** Open · P1

### CUST-04 — Delete failures are swallowed without user feedback

**Evidence:** The `confirmDelete()` catch block only resets `deleting` and does not set an error, show a toast, or render an alert. Save failures are shown in the popup, but delete failures have no corresponding feedback path.

**Impact:** A foreign-key, permission, or database failure leaves the customer in place while the operator receives no explanation. The operator may retry repeatedly or believe the record was removed when it was not.

**Recommendation:** Add a localized delete error/toast with a stable error mapping, keep the customer visible after failure, and reload only after a successful mutation. Add a rejected-delete test.

**Status:** Open · P1

### CUST-05 — Customer history is absent from the sector's customer-management flow

**Evidence:** The screen renders name, email, phone, notes, and edit/delete actions. No customer-history API call, sales-history query filtered by customer, loyalty summary, or customer detail view is present. The schema includes `sales.customer_id` and `loyalty_accounts.customer_id` relationships, but this screen does not expose them.

**Impact:** Staff cannot inspect purchase context, loyalty status, or relationship history while managing a customer. Deleting or editing a record is therefore performed without the context needed for safe customer-data operations.

**Recommendation:** Add a read-only customer detail/history view with paginated sales and loyalty summaries, scoped to the active store and permission-gated. Keep sensitive data minimized, provide loading/error/empty states, and make destructive actions available only after the relationship view has been considered.

**Status:** Open · P2 product capability/privacy

### CUST-06 — Search is an unbounded client-side privacy and performance surface

**Evidence:** `listCustomersScoped()` loads the complete customer collection and `filteredCustomers` searches name, email, and phone entirely in the browser. There is no pagination, query limit, server-side search, or field-level masking in the screen.

**Impact:** Large customer lists increase load time and memory use. More personally identifiable information is delivered to and retained in the renderer than is needed for a narrow search, increasing exposure if the renderer is logged, inspected, or compromised.

**Recommendation:** Add scoped, paginated server-side search with a bounded page size and explicit sort order. Return only fields needed for the current view, consider masking email/phone by role, and add pagination/loading tests for search and empty results.

**Status:** Open · P2 performance/privacy risk

### CUST-07 — Customer labels and fallback strings are incomplete or inconsistent across locales

**Evidence:** The component hardcodes `placeholder="Search by name, email, or phone…"`, `aria-label="Search customers"`, `aria-label="Actions"`, row labels such as ``Edit ${customer.name}`` and ``Delete ${customer.name}``, and form placeholders including `e.g. Jane Smith`, `jane@example.com`, `+1-555-0100`, and `Preferences, special notes…`. Several customer Fluent messages are attribute-only or empty in the inspected bundles, including search, action-column, row-action, placeholder, and modal-close keys. Delete failures are swallowed rather than exposed through a stable localized message.

**Impact:** Indonesian/localized users may receive English or blank labels. Literal JSX attributes can conflict with `Localized attrs`, and attribute-only messages passed to `l10n.getString()` can produce undefined values. Customer data-entry and screen-reader experiences therefore vary by locale.

**Recommendation:** Make every visible string and ARIA attribute value-bearing and bundle-complete in English and Indonesian. Use one localization owner per attribute, remove literal fallbacks, map backend errors to stable localized messages, and add a locale-parity test for the screen.

**Status:** Open · P2 accessibility/i18n

### CUST-08 — Edit/delete controls are below the touch-target convention

**Evidence:** `.customer-mgmt-action-btn` uses compact text-button padding (`var(--space-1) var(--space-2)`) without a minimum height or width. The table's action controls are therefore content-sized rather than guaranteed 44px touch targets.

**Impact:** Customer actions are difficult to operate on tablet and touch terminals, and compact adjacent Edit/Delete controls increase accidental activation risk for destructive actions.

**Recommendation:** Set a minimum 44px height and adequate horizontal hit area while retaining compact visual styling, increase action spacing on narrow viewports, and add a tablet/keyboard interaction test.

**Status:** Open · P2 UX/accessibility

### CUST-09 — Form validation is incomplete at the client boundary

**Evidence:** The client validates only that the name is non-empty before save. Email and phone are passed when non-empty and validated by the backend, but the UI provides no field-level validation or localized guidance. Notes have no length limit in the screen, and raw backend exception text is displayed for save failures.

**Impact:** Operators discover invalid contact data only after an IPC round trip, with errors not associated with the offending field. Very large notes or malformed input can create poor UX and inconsistent error presentation.

**Recommendation:** Add field-level email/phone validation using the same contract as the backend, sensible notes length limits, `aria-invalid` and error descriptions, and stable localized error mapping. Keep backend validation authoritative and test invalid email, phone, and overlong notes.

**Status:** Open · P2

### CUST-10 — Refreshes have no stale-response protection

**Evidence:** `load()` writes the customer list from whichever `listCustomersScoped()` call resolves last. It has no request generation, cancellation flag, or AbortController. The effect reloads when the session token changes, and create/update/delete handlers call `await load()` while another load may still be active.

**Impact:** A slower request from an earlier session or mutation can overwrite newer customer data. A user may see stale records or data from the previous refresh generation until another reload.

**Recommendation:** Add a request sequence guard or cancellation pattern tied to the session token and mutation generation. Add a deferred-promise test that resolves overlapping requests out of order.

**Status:** Open · P2 risk

### CUST-11 — Current tests omit critical privacy, failure, and accessibility paths

**Evidence:** `CustomerManagementScreen.test.tsx` has 15 passing tests covering rendering, loading, empty/search states, create, edit, and cancel. It does not cover scoped session arguments, list failure/retry, delete confirmation/failure, customer relationship handling, invalid email/phone, Escape/focus restoration, localized ARIA output, touch targets, or stale-load ordering.

**Impact:** Regressions in the authorization boundary, privacy behavior, recovery UX, and accessibility can pass the current test suite.

**Recommendation:** Add UI and IPC contract tests for session-scoped CRUD, failure/retry behavior, confirmation semantics, locale parity, and overlapping loads. Add backend tests for session resolution, permission ownership, and deletion with sales/loyalty references.

**Status:** Open · P3 QA gap

## Positive controls observed

- Customer listing uses the session-scoped command.
- Backend validates non-empty names and validates email/phone through foundation value types on create/update.
- Backend CRUD uses bound SQL parameters.
- Create/update permission checks exist on the legacy handlers, although their identity is caller-supplied and store scope is global.
- Loading skeleton, successful-empty state, search no-match state, and disabled in-flight controls are present.
- Shared `SettingsPopup` provides dialog semantics, focus trapping, Escape handling, and body scroll locking.
- Search matching is case-insensitive and covers name, email, and phone.

## Test and validation results

Focused validation completed during this audit:

```text
cd ui
npx vitest run src/__tests__/CustomerManagementScreen.test.tsx
npm run typecheck
```

Results:

- Focused UI tests: **15 passed, 0 failed**
- TypeScript typecheck: **passed with 0 errors**
- Report existence and Markdown trailing-whitespace validation: **passed after report generation**
- No dedicated Rust customer test count is claimed; backend source was inspected but no focused Rust test command was run during this audit

## Recommended remediation order

1. **CUST-01:** Make all customer mutations session-scoped and resolve authorization identity server-side.
2. **CUST-02 and CUST-04:** Add relationship-aware delete confirmation and visible failure recovery.
3. **CUST-03:** Separate load errors from a genuine empty customer database.
4. **CUST-05 and CUST-06:** Add scoped customer history and bounded/server-side search with privacy minimization.
5. **CUST-07 through CUST-09:** Repair localization, touch targets, and field-level validation.
6. **CUST-10 and CUST-11:** Guard refresh races and expand security/accessibility/failure coverage.

## Audit status

This is an evidence-based audit report only. No production code was changed. Findings remain **Open** until remediation commits link each item to tests and validation results.
