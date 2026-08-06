# Loyalty Module Audit — July 2026

> **Audit date:** 2026-07-31
> **Sector:** 02 — Loyalty module
> **Status:** AUDITED · LOY-01 remediated; remaining findings require follow-up
> **Scope:** Loyalty management UI, points and redemption APIs, tier configuration, persistence and migration schema, sale-completion integration, authorization, localization, theming, tests, and module documentation.

## Executive summary

The Loyalty surface has a functional management screen, tier cards, expandable account activity, checkout redemption support, transactional point writes, seeded tiers, and focused automated coverage. Validation is green: the Loyalty management screen has 17 passing Vitest tests, the `modules-loyalty` crate has 8 passing unit tests plus 1 doctest, and the filtered `oz-core` loyalty integration run passed its matching tests.

The baseline audit found critical integrity and authorization gaps behind that healthy UI. Before the LOY-01 remediation, loyalty commands operated on the process-global database, accepted raw customer and sale identifiers, and did not enforce a loyalty permission or session scope. Points earning is not idempotent by sale ID, and there is no compensating refund/void path in the loyalty event flow. Tier updates accept invalid negative or non-finite business values. The screen silently hides load failures, loads every account and customer without pagination, and relies on row-level interactive semantics that need stronger table keyboard/accessibility treatment.

## Architecture and data flow

- **Frontend:** `ui/src/features/loyalty/LoyaltyManagementScreen.tsx` and `LoyaltyManagementScreen.css`.
- **Frontend API:** `ui/src/api/loyalty.ts`.
- **Checkout integration:** `ui/src/features/sales/PaymentModal.tsx` calls loyalty account/value/redemption APIs when the feature is enabled.
- **Desktop commands:** `apps/desktop-client/src/commands/loyalty.rs`.
- **Tablet commands:** `apps/tablet-client/src/commands/loyalty.rs`.
- **Persistence:** `crates/oz-core/src/db/loyalty.rs`.
- **Schema:** `crates/oz-core/migrations/031_loyalty.sql` defines tiers, accounts, and transactions.
- **Event integration:** `platform/startup/src/event_handlers.rs::LoyaltyEarnHandler` subscribes to `SaleCompleted`; startup registers it in `platform/startup/src/lib.rs`.
- **Module POC:** `modules/loyalty/src/{models,repository,service,lib}.rs`.
- **Permissions:** `modules/loyalty/manifest.json` advertises `loyalty:view`, `loyalty:earn`, `loyalty:redeem`, and `loyalty:manage`; the remediated scoped commands enforce these permissions server-side.
- **Localization:** `ui/src/locales/loyalty.ftl` and `ui/src/locales/loyalty.id.ftl`, combined by `ui/src/i18n/index.ts`.
- **Registration:** `ui/src/features/loyalty/register.tsx` exposes the page and nav item as manager-only in the frontend registry.
- **Tests:** `ui/src/__tests__/LoyaltyManagementScreen.test.tsx`, `crates/oz-core/src/db/loyalty.rs` tests, `crates/oz-core/tests/loyalty_integration.rs`, startup handler tests, and API mock compile tests.

## Findings

### LOY-01 — Loyalty commands were global-database and unauthenticated

**Severity:** P0 — tenant isolation and financial authorization risk
**Status:** Remediated in the current working tree

**Original evidence:**

- `apps/desktop-client/src/commands/loyalty.rs` and `apps/tablet-client/src/commands/loyalty.rs` previously locked `state.db` directly for get, list, earn, redeem, tier list, tier update, points value, and account creation.
- The previous loyalty command arguments contained `customer_id`, `sale_id`, and points, but no session token or store identifier.
- The previous commands did not call `resolve_store`, `resolve_scope`, `require_permission_for_user`, or an equivalent backend authorization helper.
- The frontend page was registered with `requiredRole: 'manager'`, but UI role gating was not a security boundary.
- `ui/src/api/customers.ts` used by the page previously called the legacy global `list_customers` rather than the scoped customer list.

**Impact:** Before remediation, a caller that could invoke Tauri commands could read or mutate loyalty accounts across the process-global database without a server-side permission check. In a multi-store session, the loyalty screen could combine global loyalty data with customer data from an unrelated scope.

**Remediation evidence:**

- All eight desktop and tablet loyalty commands now accept `session_token`, resolve `(SessionContext, store connection)` through `AppState::resolve_scope`, and operate only on the session's store database.
- Read operations enforce `permissions::LOYALTY_VIEW`; earning, redemption, and tier updates enforce `LOYALTY_EARN`, `LOYALTY_REDEEM`, and `LOYALTY_MANAGE` respectively.
- Both Tauri clients register only the eight `*_scoped` loyalty commands. No legacy unscoped loyalty command registration remains.
- `ui/src/api/loyalty.ts` now exposes only scoped IPC wrappers, and `LoyaltyManagementScreen` obtains the session token from `useWorkspace()` before loading or saving.
- `PaymentModal` uses `listCustomersScoped` when a session exists and refuses to fall back to the legacy global customer list when it does not.
- `ui/src/dev-mock/tauri-api.ts` mirrors the scoped command names so browser development does not silently exercise a stale contract.
- `modules/loyalty/manifest.json` now declares the four permissions enforced by the command layer.
- `ui/src/__tests__/api-loyalty-contract.test.ts` asserts every wrapper uses a scoped command and session token; the focused PaymentModal tests assert both the no-token refusal and the session-scoped customer path.

**Remaining boundary:** `platform/startup/src/event_handlers.rs::LoyaltyEarnHandler` still uses its existing startup database connection. Its store-aware event routing and duplicate-event behavior remain tracked under LOY-02/LOY-03 and were intentionally not claimed as part of this command/API remediation.

**Validation:** `npm run typecheck`, UI lint, 61 focused Vitest tests, `cargo fmt --all -- --check`, and `cargo check -p oz-pos-app -p oz-pos-tablet` pass after the remediation.

### LOY-02 — Earning points is not idempotent by sale

**Severity:** P0 — loyalty balance inflation on retries/replays
**Status:** Open

**Evidence:**

- `crates/oz-core/src/db/loyalty.rs::earn_points` inserts a new `loyalty_transactions` row for every call and increments the account balance.
- `loyalty_transactions.sale_id` is nullable and has no unique constraint in `crates/oz-core/migrations/031_loyalty.sql`.
- There is no lookup for an existing earn transaction by `sale_id` before inserting.
- `LoyaltyEarnHandler` calls `earn_points` for every `SaleCompleted` event, and the offline/event-bus architecture can replay events during retry or recovery.
- Existing tests cover multiple distinct sale IDs but do not call the same sale twice and assert no balance change.

**Impact:** Duplicate event delivery or a command retry can award points repeatedly for one purchase. Lifetime points, current points, tier promotion, and any downstream rewards become incorrect.

**Recommendation:** Enforce idempotency transactionally using a unique projection key such as `(account_id, sale_id, txn_type)` for earn transactions, or a processed-event table keyed by sale ID. Return the existing transaction on an idempotent retry. Add duplicate-event and concurrent-replay tests.

### LOY-03 — No refund or void compensation path for earned points

**Severity:** P0 — points remain after reversed sales
**Status:** Open

**Evidence:**

- The only loyalty event subscriber found in the inspected event-handler sources is `LoyaltyEarnHandler` for `SaleCompleted` in `platform/startup/src/event_handlers.rs`.
- `earn_points` creates positive `earn` transactions and increments both `points` and `lifetime_points`.
- The loyalty transaction model documents `redeem`, `adjust`, and `expire`, but no production refund/void handler was found in the inspected sources that creates a negative compensation for a reversed sale.
- The sale/refund surfaces are separate from loyalty and no sale reversal identifier is used to reconcile an existing earn transaction.

**Impact:** A refunded or voided sale can leave spend-derived points in the customer's balance. This enables redemption of value the customer no longer earned and causes tier progression to diverge from actual net purchases.

**Recommendation:** Define the business policy for refunds and voids. Add idempotent `expire`/`adjust` compensation transactions linked to the original sale and reversal ID, with a policy for whether lifetime points and tier status can decrease. Wire the corresponding domain events and test full sale → earn → refund/void → compensation flows.

### LOY-04 — Tier updates accept invalid business values

**Severity:** P1 — configuration can corrupt earning behavior
**Status:** Open

**Evidence:**

- `LoyaltyManagementScreen.tsx` validates only `NaN` and empty name. It does not reject negative `min_points`, negative `points_per_unit`, negative/zero multipliers, malformed colour values, duplicate thresholds, or tiers out of order.
- `apps/desktop-client/src/commands/loyalty.rs::update_loyalty_tier` forwards all values directly to `Store::update_tier` with no validation or permission check.
- `Store::update_tier` executes the update without checking ranges, ordering, colour format, or whether at least one tier remains at zero points.
- The earn calculation uses `saturating_mul`, floating-point conversion, and rounding, so extreme or invalid tier values can produce unexpected point amounts.

**Impact:** A manager can accidentally configure negative or nonsensical earning rates, prevent tier assignment, or create unexpected rewards. Invalid settings can be written through direct IPC even if the visible form is improved later.

**Recommendation:** Validate at the domain/command boundary and in the database transaction: names non-empty, thresholds non-negative and strictly ordered, points rate positive, multiplier finite and within an explicit business range, and colour a valid approved format. Return field-specific errors and add tests for every invalid boundary.

### LOY-05 — Load failures are silently presented as stale or empty data

**Severity:** P1 — operational and decision-making risk
**Status:** Open

**Evidence:**

- `LoyaltyManagementScreen.tsx::load` catches all errors with `/* API unavailable — keep existing state */` and never sets the `error` state.
- `loading` is set false in `finally`; if the initial request fails, the screen can render the empty account state and an empty tier tab.
- The existing `error` state is used only for tier-save validation/API failures.
- There is no localized retry control or alert describing which of the three parallel requests failed.

**Impact:** An authorization failure, offline condition, or IPC/database error can look like “No loyalty accounts yet” or an unchanged stale list. Operators may incorrectly assume the program has no members or that changes were saved.

**Recommendation:** Track initial-load and save errors separately. Render a localized `role="alert"` with retry and preserve a successful empty state only after all requests resolve successfully. Consider independent loading/error states for accounts, customers, and tiers so a tier failure does not hide valid account data.

### LOY-06 — Unbounded list loading and per-account transaction queries will not scale

**Severity:** P1 — performance and availability risk
**Status:** Open

**Evidence:**

- The page loads all loyalty accounts, all customers, and all tiers at mount with `Promise.all`.
- `Store::list_loyalty_accounts` selects every account and then executes a separate recent-transaction query for each account (`LIMIT 5`), creating an N+1 query pattern.
- The UI renders the complete result as one table with no pagination, search, server-side filtering, or virtualized rows.
- `list_customers` also returns the complete customer table for name mapping.

**Impact:** Large stores incur increasing SQLite work, IPC payload size, memory usage, and React render cost. One slow or failing dataset blocks the whole initial view.

**Recommendation:** Add paginated, store-scoped list endpoints with search/filter/sort and a bounded customer-name join or projection. Replace N+1 transaction queries with a batched/window query or load details only when a row expands. Add performance tests at representative account counts.

### LOY-07 — Checkout redemption has no explicit sale/amount/currency binding at the loyalty boundary

**Severity:** P1 — discount integrity risk
**Status:** Open

**Evidence:**

- `redeem_points` accepts only `customer_id`, `points`, and `sale_id`; it calculates `discount_minor = points * 1` without receiving or checking the sale total, currency, or remaining payable amount.
- The command trusts the supplied `sale_id` and does not verify that the sale belongs to the customer, is active/completable, or has not already received a redemption transaction.
- The transaction schema allows multiple rows for the same sale and has no unique redemption key.
- `get_points_value` accepts any signed integer and returns a negative value for negative points; it has no domain validation.

**Impact:** A caller can potentially redeem points against an unrelated sale or apply a discount larger than the payable balance, depending on PaymentModal enforcement. Repeated redemption attempts can produce multiple deductions for one checkout.

**Recommendation:** Make redemption a single server-side operation that loads and locks the sale, verifies ownership/customer association, status, currency, remaining total, and idempotency, then caps the discount. Reject negative values in `get_points_value` and require a positive, bounded redemption request. Add tests for duplicate sale redemption, sale mismatch, over-discount, and retry.

### LOY-08 — Tier progression and point semantics are underspecified and inconsistent across docs/schema/tests

**Severity:** P2 — business-rule drift
**Status:** Open

**Evidence:**

- `031_loyalty.sql` seeds Silver at 100 lifetime points, Gold at 500, and Platinum at 2000.
- `crates/oz-core/tests/loyalty_integration.rs` comments describe Silver at 500, Gold at 2500, and Platinum at 10000 in the auto-upgrade test, while the actual seeded thresholds differ.
- `modules/loyalty/src/models.rs` describes `points_per_unit` as base points per minor unit, while the implementation divides by 100 after multiplying; this should be documented as per major currency unit or otherwise renamed.
- The points-value comment says 100 points = 100 minor units ($1.00), while the fixed ratio constant is 1 and the UI/API do not expose a configurable policy.

**Impact:** Administrators and developers can reason about different programs than the one executing in production. Threshold changes can silently alter customer status, while comments and tests may not detect the intended policy change.

**Recommendation:** Establish one versioned loyalty policy: earning unit, rounding mode, tier thresholds, currency conversion, and downgrade/refund behavior. Use named domain constants/configuration, update schema comments/docs/tests, and add boundary tests immediately below/at/above every tier threshold.

### LOY-09 — Loyalty module ownership and documentation are stale or contradictory

**Severity:** P2 — architecture and maintenance risk
**Status:** Open

**Evidence:**

- `modules/loyalty/src/lib.rs` says commands are TBD, the frontend is `ui/src/features/crm/`, and API/locale locations are TBD, although the live files are `apps/*/src/commands/loyalty.rs`, `ui/src/features/loyalty/`, `ui/src/api/loyalty.ts`, and `ui/src/locales/loyalty*.ftl`.
- The module README says version `0.0.19`, while the current repository release is 0.0.24.
- The README permissions are `loyalty:read`, `loyalty:write`, and `giftcards:issue`; the manifest lists `loyalty:view` and `loyalty:manage`; neither set is enforced by the current loyalty commands.
- The module repository/service exposes only a small account lookup and gift-card lookup, while the live loyalty behavior remains in `oz-core`.

**Impact:** Documentation points contributors to nonexistent or future paths and creates uncertainty about the authoritative permission and implementation contract.

**Recommendation:** Mark the module explicitly as a POC or complete the migration. Correct paths, version, feature ownership, and permission names; document the actual command boundary and event wiring. Add a docs drift check for module manifests and owned paths.

### LOY-10 — Accessibility and interaction semantics need hardening

**Severity:** P2 — keyboard/screen-reader usability gap
**Status:** Open

**Evidence:**

- The accounts table row is a `tr` with `role="button"`, `tabIndex={0}`, `aria-expanded`, and an `aria-label` that only says “Expand” or “Collapse”; the row has no stable name describing which customer/account is being expanded.
- The expand button is nested inside the clickable row; its click bubbles to the row's click handler, creating redundant and fragile interaction semantics even though the button does not define a separate handler.
- The tier tabs are ordinary buttons without tablist/tab/selected semantics.
- Tier edit inputs are wrapped in `Localized` blocks with empty children (`{ }`) and attribute-only messages; this pattern is brittle and deserves direct accessible-name assertions.
- Transaction dates use `new Date(...).toLocaleDateString()` without an explicit locale or time zone policy.

**Impact:** Screen readers may announce ambiguous controls, keyboard users may not know which account will expand, and nested row/button interactions can produce surprising state changes. Dates can vary by machine locale, making operational history inconsistent.

**Recommendation:** Use a dedicated expand button per row with a customer-specific localized label and `aria-controls`, remove row-button semantics or make row activation purely delegated, and implement proper tablist semantics. Use the app locale/timezone formatter for transaction dates. Add keyboard and accessibility tests for focus, expansion, tabs, and localized labels.

### LOY-11 — Hardcoded and inline presentation values bypass the theme/token contract

**Severity:** P2 — theming and contrast risk
**Status:** Open

**Evidence:**

- `LoyaltyManagementScreen.tsx` uses inline `style` values for tier border/background colours and hardcodes `color: '#fff'` for badges.
- The same arbitrary tier colour is applied as a background without checking contrast for white text.
- The loading skeleton uses an inline `borderRadius` style rather than a CSS class.
- The page contains user-visible fallback literals such as `Failed to save tier`, `Edit`, `Cancel`, `Save`, and `—` outside a fully consistent localization contract.

**Impact:** Custom tier colours can produce unreadable badges in light or dark themes, and token-compliance tooling cannot reason about the inline styling. English or inconsistent fallback text can leak into Indonesian.

**Recommendation:** Keep dynamic tier colour in a validated CSS custom property, compute a readable foreground colour or use an accessible contrast-preserving badge treatment, and move static styling into CSS classes. Ensure every fallback string is Fluent-backed with a real message value and add theme/contrast compliance tests.

### LOY-12 — Concurrent account creation and focused tests omit high-risk paths

**Severity:** P2 — concurrency and regression-detection gap
**Status:** Open

**Evidence:**

- `get_or_create_loyalty_account` first checks for an account and then inserts it, relying on the `UNIQUE` constraint on `customer_id`; the check and insert are not performed as one atomic insert-or-ignore/upsert operation.
- Concurrent account creation for the same customer can therefore race and return a SQLite constraint error instead of the existing account. No concurrent account-creation test was found.
- The focused UI suite has 17 passing tests covering normal rendering, empty/loading states, expansion, tier editing, validation, cancel, and successful save.
- It does not cover list rejection/retry, partial parallel-load failure, duplicate expansion click behavior, nested expand-button behavior, localized attributes, permission denial, or touch/contrast behavior.
- Core loyalty tests cover normal earn/redeem and several point-size boundaries, but not duplicate sale IDs, refunds/voids, negative sale totals, currency, overflow/extreme tier values, duplicate redemption, or authorization/session isolation.
- `modules-loyalty` tests cover lifecycle only; they do not exercise the actual loyalty persistence path.

**Recommendation:** Make account creation atomic and idempotent, returning the existing account after a concurrent insert race. Add a concurrency regression test. Then add tests in risk order: idempotent earn, refund compensation, redemption binding, invalid tier values, permission/session isolation, load retry, and keyboard semantics. Make the persistence and command integration tests required gates for loyalty changes.

## Positive observations

- Point earn and redemption balance writes are wrapped in SQLite transactions.
- Customer accounts are uniquely constrained in the schema (`loyalty_accounts.customer_id UNIQUE`).
- Foreign keys link accounts to customers, transactions to accounts, and transactions to sales.
- The UI has a loading skeleton, successful empty state, expandable recent transaction details, tier edit cancellation, and a `role="alert"` for tier-save validation errors.
- The expand button has the project touch-target minimum, and the tier edit button also sets a minimum touch height.
- Point calculations include explicit small-total tests and use rounding rather than silent integer truncation.
- Focused validation passed: 17/17 LoyaltyManagementScreen tests, 8/8 `modules-loyalty` tests, 1/1 module doctest, and the matching `oz-core` loyalty test filters.

## Recommended implementation order

1. **Security and scope:** ✅ session-resolve all loyalty commands and enforce view/earn/redeem/tier-management permissions (LOY-01 remediated).
2. **Financial correctness:** make earn/redeem idempotent and bind operations to validated sales; add refund/void compensation.
3. **Tier policy validation:** enforce non-negative, finite, ordered tier configuration and publish one canonical policy.
4. **Operational resilience:** expose load errors and retry, then add pagination/batched transaction loading.
5. **Accessibility and theming:** fix nested row/button semantics, tab roles, localized names/dates, dynamic colour contrast, and inline styling.
6. **Ownership and quality:** reconcile the module POC with live implementation, refresh docs, and add the missing integration/security tests.

## Validation performed

### Original audit baseline

- `cd ui && npx vitest run src/__tests__/LoyaltyManagementScreen.test.tsx` — **17 passed, 0 failed**.
- `cargo test -p modules-loyalty` — **8 unit tests passed, 0 failed; 1 doctest passed**.
- `cargo test -p oz-core loyalty -- --nocapture` — matching loyalty filters passed; the command reported **24 oz-core tests, 10 customer integration tests, and 3 loyalty integration tests** passing with no failures.

### LOY-01 remediation validation

- `cd ui && npx vitest run src/__tests__/api-loyalty-contract.test.ts src/__tests__/LoyaltyManagementScreen.test.tsx src/__tests__/PaymentModalEdgeCases.test.tsx src/__tests__/MockFactoriesCompile.test.tsx` — **61 tests passed, 0 failed** across 4 files.
- `cd ui && npm run typecheck` — passed.
- `cd ui && npm run lint -- --no-fix` — passed.
- `cargo fmt --all -- --check` — passed.
- `cargo check -p oz-pos-app -p oz-pos-tablet` — passed.
- `cargo test -p oz-pos-app commands::loyalty --lib` — **8 passed, 0 failed**, including direct Tauri-state tests for invalid sessions, denied permissions, and store isolation.
- `cargo test -p oz-pos-tablet commands::loyalty --lib` — **8 passed, 0 failed**, with the same direct command-boundary coverage.
- `cargo test -p oz-pos-app commands::authz --lib` — **4 passed, 0 failed**, including inactive-user denial.
- `cargo test -p oz-pos-tablet commands::authz --lib` — **4 passed, 0 failed**, including inactive-user denial.
- `cargo test -p oz-pos-app state::tests --lib` — **9 passed, 0 failed**, including expired-session removal and store-database isolation.
- `cargo test -p oz-pos-tablet state::tests --lib` — **8 passed, 0 failed**, including expired-session removal and store-database isolation.
- Source inspection confirmed no remaining production registration or frontend IPC call uses the legacy unscoped loyalty command names.
- Source inspection covered the UI, CSS, API client, desktop/tablet commands, core loyalty persistence, migration schema, startup event wiring, module POC, locales, permissions, checkout integration, tests, and documentation.

## Fix status

LOY-01 is **remediated and validated in the current working tree**. The remediation changes the desktop/tablet command boundary, frontend API/callers, development IPC mock, loyalty permission manifest, and focused tests. LOY-02 through LOY-12 remain **Open** and require separate implementation work; in particular, the startup event handler's store-aware routing and duplicate-event behavior were not silently marked complete by this change.

> **2026-08-06 — LOY-10 partial remediation (accessible name):** the account expand row and its nested expand button now expose a customer-specific accessible name (`loyalty-expand-account` / `loyalty-collapse-account` with `{ $name }`, en + id), and the nested button received a real `onClick` handler instead of relying on click bubbling. Pinned by the `names the expand control with the customer (LOY-10)` vitest. Remaining LOY-10 items (tablist semantics for tier tabs, localized transaction dates) stay open.
