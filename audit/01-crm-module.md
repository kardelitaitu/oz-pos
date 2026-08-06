# CRM Module Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** 01 — CRM module
> **Status:** PARTIALLY REMEDIATED · CRM-01 resolved; CRM-02–CRM-11 remain open
> **Scope:** Customer management UI, customer APIs and Tauri commands, CRM module code, persistence and migrations, event wiring, localization, authorization, tests, and module documentation.

## Executive summary

The CRM surface has a usable customer CRUD screen, strong baseline validation, tokenized styling, and a transactional sale-completion handler. Focused validation is green: the customer screen has 16 passing Vitest tests, the scoped API contract has 4 passing tests, and the CRM crate has 14 passing unit tests plus 1 passing doctest.

The audit found two high-impact production risks. First, the customer list was store-scoped on desktop, but create/update/delete commands used the process-global database and accepted only a caller-supplied `user_id`; this was inconsistent with ADR #7 and could mutate the wrong store in a multi-store session. CRM-01 is now remediated for the customer-management path with session-scoped commands. Second, the scoped list command does not enforce `customers:view`, so a valid session can potentially enumerate customer records without the matching backend permission. The screen also hides load and delete failures, and the CRM history promised by the module documentation is currently only aggregate spending/points updates rather than a customer-visible purchase-history feature.

## Architecture and data flow

- **Frontend:** `ui/src/features/customers/CustomerManagementScreen.tsx` and its tokenized stylesheet.
- **Frontend API:** `ui/src/api/customers.ts`.
- **Desktop commands:** `apps/desktop-client/src/commands/customers.rs`.
- **Tablet commands:** `apps/tablet-client/src/commands/customers.rs`.
- **Persistence:** `crates/oz-core/src/db/customers.rs`.
- **Schema:** `crates/oz-core/migrations/007_customers.sql` creates `customers`; `042_customer_id_on_sales.sql` links sales to customers.
- **Module POC:** `modules/crm/src/{models,repository,service,handlers,lib}.rs`.
- **Event wiring:** `platform/startup/src/lib.rs` subscribes `CrmHistoryHandler` to `sale.completed`.
- **Authorization:** `platform/core/src/rbac.rs` defines `customers:create`, `customers:view`, `customers:edit`, and `customers:delete`.
- **Localization:** `ui/src/locales/customers.ftl` and `ui/src/locales/customers.id.ftl`.
- **Tests:** `ui/src/__tests__/CustomerManagementScreen.test.tsx`; CRM module tests and customer integration tests.

## Findings

### CRM-01 — Store-scoped list conflicts with global CRUD commands

**Severity:** P1 — high data-integrity / tenant-isolation risk
**Status:** Resolved — 2026-07-31

**Evidence:**

- `ui/src/api/customers.ts` calls `list_customers_scoped` with `sessionToken`.
- Desktop `list_customers_scoped` resolves the store from `session_token` and reads through that store connection (`apps/desktop-client/src/commands/customers.rs`).
- Desktop now exposes `create_customer_scoped`, `update_customer_scoped`, and `delete_customer_scoped`; each resolves the store and authenticated user from `session_token` before checking permissions and mutating that store connection.
- Tablet now exposes the same scoped list and mutation commands, backed by its own `StoreDatabaseManager`; both clients register the scoped commands instead of the legacy mutation commands.
- The customer screen and API now pass the active workspace session token and never send a caller-supplied `user_id` for customer mutations.
- `modules/crm/src/lib.rs` explicitly documents that the module's backend remains split between `oz-core` and client command locations.

**Impact before remediation:** In a multi-store deployment, the list could come from the session-resolved store while mutations went to the global database. A successful create/update/delete could therefore be invisible after refresh or affect the wrong store.

**Resolution:** The migrated customer-management path now resolves both store and user identity server-side from the opaque session token. `ui/src/api/customers.ts` provides only scoped mutation bindings, `CustomerManagementScreen.tsx` uses them for create/update/delete, and desktop/tablet command registration no longer exposes the legacy mutation handlers. The legacy Rust functions remain as documented compatibility code but are not registered as Tauri commands. The shared API contract suite covers all three command names and payloads; the screen suite verifies the session token is passed on list/create/update. Cross-store integration testing remains a follow-up hardening item.

### CRM-02 — Customer listing does not enforce the view permission

**Severity:** P1 — authorization gap
**Status:** Open

**Evidence:**

- `platform/core/src/rbac.rs` defines `permissions::CUSTOMERS_VIEW`.
- `apps/desktop-client/src/commands/customers.rs::list_customers_scoped` resolves the session and lists customers but does not call `require_permission_for_user(..., CUSTOMERS_VIEW)`.
- The same command has no user-derived permission check before returning the complete customer list.
- The scoped create, update, and delete commands derive the user identity from the session. The unregistered legacy command implementations still accept `args.user_id` for compatibility, but are no longer exposed through either Tauri invoke handler.

**Impact:** Backend authorization does not match the declared permission model. A user with a valid session but without customer-view permission may be able to enumerate customer names, email addresses, phone numbers, and notes.

**Recommendation:** Require `CUSTOMERS_VIEW` in every read command, including scoped list and get. Resolve the user identity from the session token rather than trusting a frontend-supplied user ID. Add command-level tests for cashier/custom-role denial and manager/owner access.

### CRM-03 — Load failures are silently rendered as an empty customer database

**Severity:** P1 — misleading recovery behavior
**Status:** Open

**Evidence:**

- `CustomerManagementScreen.tsx` sets `loading` false in `load()`'s `finally` block.
- The `catch` block is empty (`// IPC unavailable.`) and does not set `error`.
- Rendering then checks `customers.length === 0` and shows “No customers yet.”, even when the request failed.
- There is no retry action or error state in the screen.

**Impact:** A disconnected, unauthorized, or failed store request can look like a legitimate empty database. Operators may create duplicate records or assume customer data was deleted.

**Recommendation:** Preserve a typed load error separately from the empty state. Render a localized error panel with a retry button and keep the empty state only for a successful zero-row response. Add an accessible `role="alert"` and an `aria-live` status for retry progress.

### CRM-04 — Delete is immediate and delete failures are invisible

**Severity:** P1 — destructive UX and recovery gap
**Status:** Open

**Evidence:**

- Each row's Delete button calls `confirmDelete(customer.id)` directly; there is no confirmation dialog.
- `confirmDelete` catches errors, clears `deleting`, and otherwise provides no user-visible error.
- The screen has an `error` state for save validation, but delete errors do not use it.
- The focused UI suite covers rendering, search, create, edit, and cancel, but has no delete-confirmation or delete-failure test.

**Impact:** A single accidental click permanently removes a customer record (subject to backend behavior), and a failed deletion gives no explanation or retry path.

**Recommendation:** Add a localized confirmation dialog with the customer name, explicit destructive action, Escape handling, focus trapping, and a loading state. Surface delete failures in an alert/toast and preserve the row. Add tests for cancel, confirm, failure, and keyboard activation.

### CRM-05 — “Purchase history” is documented but not exposed as customer history

**Severity:** P1 — incomplete feature contract
**Status:** Open

**Evidence:**

- `modules/crm/README.md` says CRM handles “purchase history.”
- `modules/crm/src/handlers.rs::CrmHistoryHandler` responds to `SaleCompleted` by incrementing `total_spent_minor` and `loyalty_points`.
- The `customers` schema contains aggregate totals but no customer-history table or sale-history query owned by CRM.
- `CustomerManagementScreen.tsx` renders only name, email, phone, notes, and edit/delete actions; it has no history view or customer-history API call.
- `ui/src/api/customers.ts` exposes CRUD and `getCustomer`, but no history endpoint.

**Impact:** Users cannot inspect the transactions that explain a customer's spend or loyalty balance. The documentation and manifest overstate the current UI/API capability, making the feature appear complete when only aggregation is implemented.

**Recommendation:** Decide and document the contract: either rename the current capability to aggregate customer metrics, or implement a scoped history endpoint backed by sales/customer joins (with pagination, date filters, and authorization) and a customer detail/history view. Add tests linking completed sales to visible history rows.

### CRM-06 — Sale-completion aggregation is not idempotent and does not validate currency

**Severity:** P1 — financial/loyalty integrity risk
**Status:** Open

**Evidence:**

- `CrmHistoryHandler::handle` performs a read-modify-write of `total_spent_minor` and `loyalty_points` for every `SaleCompleted` event.
- There is no processed-event or sale-ID uniqueness check before applying the increment. Re-delivery of the same event will count the sale again.
- Points are calculated as `event.total_minor / 100` and the amount is added directly to the customer's `total_spent_minor`.
- `Customer` stores a `currency`, and `SaleCompleted` also carries a `currency`, but the handler does not compare them or convert the amount.

**Impact:** Event retries, duplicate publications, or recovery replays can inflate customer spend and points. A sale in a different currency can be added as if it were in the customer's stored currency.

**Recommendation:** Add an idempotency record keyed by sale ID (or a unique CRM projection table) and make the update transactional with that record. Define currency policy explicitly: reject mismatches, convert using a recorded exchange rate, or maintain per-currency totals. Add duplicate-event, negative/refund, overflow, and currency-mismatch tests.

### CRM-07 — CRM module and core persistence have duplicate, incomplete ownership

**Severity:** P2 — architectural drift / maintenance risk
**Status:** Open

**Evidence:**

- `modules/crm/src/models.rs` defines a CRM `Customer` model separate from `oz_core::Customer` used by the client commands.
- `modules/crm/src/repository.rs` implements only `get_customer` and transactional `create_customer_tx`; update, delete, and list remain in `crates/oz-core/src/db/customers.rs`.
- `modules/crm/src/service.rs` exposes only get/create.
- `modules/crm/src/lib.rs` says the module is a registration/configuration layer and that physical migration is future work, while `platform/startup/src/lib.rs` already wires the CRM history handler.
- `modules/crm/manifest.json` advertises CRM permissions as `crm:view` and `crm:edit`, while the live RBAC catalog uses `customers:view`, `customers:create`, `customers:edit`, and `customers:delete`.

**Impact:** Future contributors can update one model/repository and leave the live path unchanged. The manifest's permission names do not describe the permissions enforced by the commands, and the module's advertised ownership is not the runtime ownership.

**Recommendation:** Choose one source of truth. Complete the CRM module migration (list/get/create/update/delete, DTO boundary, authorization contract, and tests) or formally mark it as a POC and remove misleading ownership claims from the manifest/README. Align manifest permissions with the live RBAC catalog or introduce a documented mapping.

### CRM-08 — Indonesian locale is incomplete for the current screen contract

**Severity:** P2 — localization/accessibility regression
**Status:** Open

**Evidence:**

- `ui/src/locales/customers.ftl` defines `customer-mgmt-table-aria = Customers`.
- `ui/src/locales/customers.id.ftl` has no corresponding `customer-mgmt-table-aria` key.
- The screen calls `l10n.getString('customer-mgmt-table-aria')` directly for the table label.
- `customers.id.ftl` also contains unrelated trailing “Category” comments/sections, suggesting the bundle has accumulated cross-feature content and needs hygiene review.

**Impact:** The table label can fall back to the source language or become unavailable depending on bundle fallback configuration, while other labels are Indonesian. This is especially visible to screen-reader users.

**Recommendation:** Add the missing key to the Indonesian bundle, run the bundle-parity and Fluent duplicate checks, and audit all locale bundles for the complete `customer-mgmt-*` key set. Keep feature-specific keys in the feature bundle.

### CRM-09 — Several localized attributes retain hardcoded English fallbacks

**Severity:** P2 — i18n compliance / accessibility risk
**Status:** Open

**Evidence:**

- `CustomerManagementScreen.tsx` contains literal `placeholder="Search by name, email, or phone…"` and `aria-label="Search customers"` inside a `Localized` wrapper.
- Row actions contain literal `aria-label={`Edit ${customer.name}`}` and `aria-label={`Delete ${customer.name}`}` inside `Localized` wrappers.
- The project audit convention requires all user-facing strings, including ARIA labels and placeholders, to be Fluent-backed; the source literals can also become the rendered value if localization attributes fail to apply.

**Impact:** English can leak into Indonesian (and future locales), and attribute-only localization failures are difficult to detect in visual testing.

**Recommendation:** Use a stable fallback only where the component contract requires it, preferably sourced from the localized message with an explicit fallback, and add a test that asserts localized placeholder and action labels in the Indonesian bundle. Include these keys in automated attribute-only/message-value compliance checks.

### CRM-10 — Row action touch targets are likely below the POS accessibility target

**Severity:** P2 — touch usability
**Status:** Open

**Evidence:**

- `.customer-mgmt-action-btn` uses `padding: var(--space-1) var(--space-2)` and `font-size: var(--text-xs)` with no minimum width or height.
- The screen places two compact actions in the table's final column, and the CSS does not provide a mobile-specific expansion.
- The existing action buttons therefore depend on text line-height for their hit area and are likely smaller than the project's 44px touch target convention.

**Impact:** Edit/Delete are difficult to activate accurately on tablet or touch POS hardware, especially in dense rows.

**Recommendation:** Apply the shared touch-target minimum (or a project-approved compact exception) with `min-height`, horizontal spacing, and responsive wrapping. Add a touch-target compliance test or computed-size assertion for this screen.

### CRM-11 — CRM UI test coverage omits failure, authorization, and destructive paths

**Severity:** P2 — regression detection gap
**Status:** Open

**Evidence:**

- `CustomerManagementScreen.test.tsx` has 15 passing tests covering loading skeleton, empty/search states, rendering, create, edit, and cancel.
- It does not cover list rejection, retry, delete confirmation/failure, save rejection rendering, locale parity, or keyboard/focus behavior of the shared modal.
- CRM Rust tests cover handler accumulation and lifecycle, but do not cover duplicate `SaleCompleted` delivery, currency mismatch, permission enforcement, or cross-store command isolation.

**Recommendation:** Add tests alongside each remediation: request failure/retry, delete flow, permission denial, session-scoped mutation isolation, event idempotency, currency policy, and localized attributes. Keep the focused tests as a required CI gate.

## Positive observations

- The UI has explicit loading skeleton, successful-empty, and search-no-match states.
- Search normalizes case and searches name, email, and phone.
- Create/update validate required names in the UI and validate email/phone again in the Rust commands.
- Database writes in `oz_core` use parameterized SQL; the CRM history update uses a transaction and checked arithmetic.
- Desktop scoped listing resolves the database from an opaque session token rather than accepting a store ID from the frontend.
- `SettingsPopup` provides `role="dialog"`, `aria-modal="true"`, Escape handling, focus trapping, portal rendering, and tokenized styling.
- Focused validation passed: 16/16 CustomerManagementScreen tests, 4/4 scoped customer IPC contract tests, and 14/14 CRM module tests plus 1/1 doctest.

## Recommended implementation order

1. **Tenant and authorization boundary:** make every customer command session-scoped and enforce `CUSTOMERS_VIEW`; derive user identity server-side.
2. **Failure safety:** add load/delete error states, retry, and delete confirmation before further feature work.
3. **Financial projection integrity:** make CRM sale processing idempotent and define currency/refund behavior.
4. **History contract:** implement customer purchase history or correct the module documentation and product wording.
5. **Localization and touch UX:** close locale parity, remove brittle attribute fallbacks, and size row actions for touch.
6. **Ownership cleanup and tests:** consolidate the module/core implementation and add isolation, failure, authorization, and event-replay coverage.

## Validation performed

- `cd ui && npx vitest run src/__tests__/api-customers-contract.test.ts src/__tests__/CustomerManagementScreen.test.tsx` — **20 tests passed, 0 failed** (4 IPC contract tests + 16 screen tests).
- `cd ui && npm run typecheck` — **passed with 0 errors**.
- `cargo check -p oz-pos-app -p oz-pos-tablet` — **passed**.
- `cargo fmt --all` — **passed**.
- `cargo test -p modules-crm` — **14 unit tests passed, 0 failed; 1 doctest passed**.
- Source inspection covered the UI, CSS, API client, desktop/tablet commands, core customer persistence, CRM module, migrations, startup event wiring, RBAC catalog, locale bundles, and module documentation.

## Fix status

CRM-01 is **Resolved** in the session-scoped customer-management path. CRM-02 and the remaining findings remain open and are intentionally not changed by this implementation.
