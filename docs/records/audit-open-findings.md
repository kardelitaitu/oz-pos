# Audit Open Findings — Consolidated

> **Source of truth for still-open audit findings.** The per-sector audit
> reports in `audit/` were consolidated into this file (and the sector
> reports removed). Each finding below keeps its original ID so the commit
> history and code comments that reference it still resolve.
>
> Fully-remediated sectors (✅ in the registry) are **not** repeated here —
> their findings are closed by the commits recorded in the deleted reports.
> Only sectors with open or partially-remediated findings survive here.

---

## CRM (`01-crm-module.md` — PARTIALLY REMEDIATED)

**Status:** CRM-01–CRM-11 ALL closed as of 2026-08-31 (CRM-06 was real and fixed same day; the rest verified fixed against current code).

Key items:
- ~~**CRM-02** — Customer listing does not enforce the view permission~~ — **VERIFIED FIXED 2026-08-31** (with one residual closed same day): `list_customers_scoped`/`search_customers_scoped` enforce `customers:view` on both clients (denial-tested). **Residual found and fixed (`7967cc2d`):** the tablet still registered the legacy unguarded `get_customer` (no session, no permission, global db — cross-store read by id); replaced with `get_customer_scoped` (gated, store-scoped, denial-tested on both clients), and the dead legacy UI wrappers were removed so no caller can reach an unregistered command.
- ~~**CRM-03** — Load failures are silently rendered as an empty customer database~~ — **VERIFIED FIXED 2026-08-31**: `loadError` state; the error view replaces the empty state when the list fails to load (`CustomerManagementScreen.tsx:465`).
- ~~**CRM-04** — Delete is immediate and delete failures are invisible~~ — **VERIFIED FIXED 2026-08-31**: `ConfirmDialog` gates deletion (CUST-02) and a localized toast surfaces delete failures (CUST-04), both tested.
- ~~**CRM-05** — "Purchase history" documented but not exposed~~ — **VERIFIED FIXED 2026-08-31**: `get_customer_history_scoped` + in-screen history view with load-failure retry (CUST-05 tested).
- ~~**CRM-06** — Sale-completion aggregation is not idempotent and does not validate currency~~ — **REAL, FIXED 2026-08-31 (`23b78594`+`841448ca`, unsubscription follow-up next commit)**: the handler WAS live — `platform/startup` subscribed `CrmHistoryHandler` to `sale.completed` in both shipping clients (an earlier grep that missed `platform/` produced a wrong "zero production writers" claim, corrected here). So the original bugs were production-real: no idempotency (event re-delivery double-counted spend) and no currency validation (foreign-currency sales added raw). The projection moved into the completion transaction (base currency, statement-level atomic increment, replay-safe via the finalize `changed==1` CAS — idempotency by construction) and `create_refund` reverses it proportionally at the sale-recorded rate (integer round-half-up, floor at zero). The handler's bus subscription is removed with it — running both writers would double-count every sale. 5 tests, Red-first.
- ~~**CRM-07** — Duplicate, incomplete ownership between CRM module and core persistence~~ — **RESOLVED 2026-08-31 by deletion + unsubscription**: the duplicate writer (`modules/crm/src/handlers.rs` + its `platform/startup` subscription) is gone; `Store` completion/refund is the single owner of the spend projection.
- ~~**CRM-08** — Indonesian locale incomplete~~ — **VERIFIED FIXED 2026-08-31**: `customers.ftl`/`customers.id.ftl` at 56/56 key parity, enforced by the i18n lint + bundle-parity pre-commit gates.
- ~~**CRM-09** — Hardcoded English fallbacks~~ — **VERIFIED FIXED 2026-08-31**: screen uses `requiredLocalized`/`getString` throughout; remaining `??` defaults are data values (empty strings, em-dash), not user-facing English.
- ~~**CRM-10** — Row action touch targets below POS minimum~~ — **VERIFIED FIXED 2026-08-31**: `.customer-mgmt-action-btn` carries `min-height/min-width: 2.75rem` (44px).
- ~~**CRM-11** — Test coverage omits failure/authorization/destructive paths~~ — **VERIFIED FIXED 2026-08-31**: 35 screen tests including delete-failure toast, load-failure retry, history retry; command-level permission denial tests on both clients (pre/post `7967cc2d`).

---

## Loyalty (`02-loyalty-module.md` — AUDITED)

**Status:** LOY-01 remediated; **LOY-06 CLOSED 2026-08-30** (earn now fires atomically at completion); SF-01 closed with it; **LOY-03 CLOSED 2026-08-31** (proportional refund reversal in-tx; void path proven unreachable + `void_sale` race fixed as CAS); LOY-04 verified fixed. Remaining: LOY-05.

Key open items:
- ~~**LOY-02** — Earning points is not idempotent by sale~~ — **VERIFIED FIXED 2026-08-30**: migration 128 enforces a unique earn/redeem projection index (`crates/oz-core/src/db/loyalty_tests.rs:556`).
- ~~**LOY-06** (P1) — loyalty earning never fires in production~~ — **CLOSED 2026-08-30**, landed in `3c23e47b` (swept by a concurrent website commit — content verified intact in HEAD; attribution recorded in the journal). Wiring (user decision: backend-atomic, base-currency): `earn_points` refactored into a connection-bound core `earn_points_with_conn` that joins the caller's transaction; `finalize_sale`/`finalize_sale_in_tx` award inside the same tx as the pending→completed transition (`changed == 1` guards replays; unique index guards races); the shortfall retry awards inline. Award uses `base_total_minor` when the CUR-02 snapshot is present (the formula is currency-naive — a low-exponent charge currency would multiply rewards). Failures logged non-fatal: a captured payment never rolls back over points. Pinned by 7 new core tests.
- ~~**SF-01** (P1, found during the LOY-06 sweep) — shortfall retry sales stuck at `pending`~~ — **CLOSED 2026-08-30**, same commit: `complete_sale_with_resolved_shortfalls` wrote `status='pending'` + a 30-min expiry and nobody finalized retry sales — invisible to every report (they filter `status='completed'`) and an auto-void time bomb once the ADR-20 reaper is wired (the reaper is NOT currently wired on either client, so today's symptom is permanent-pending). The retry settles an already-captured payment → now writes `completed`, no expiry, returns `Completed`. Follow-up UX gap (open, minor): the dialog's `onComplete` still doesn't print a receipt the way the main path does.
- ~~**LOY-03** — No refund or void compensation path for earned points~~ — **CLOSED 2026-08-31** (landed inside foreign commit `01d3932e` — swept while committing; content verified intact in HEAD): `create_refund` now reverses `round(award × refund/sale)` **in the same DB transaction**, capped at the not-yet-reversed remainder (cumulative refunds can never claw back more than the award). Ledger row records the full deduction (negative points, type `refund_reversal`, same sign convention as `redeem`); balance floors at zero (spent points aren't dragged negative); lifetime drops → tier demotion recomputes; `customers.loyalty_points` projection maintained. Idempotent per refund via deterministic PK `loyalty-reversal-<refund_id>`. Loyalty failure warns, never blocks the refund (policy matches the LOY-06 award hook). Semantics chosen by the user: proportional reversal. 8 tests (6 unit + 2 wiring). **Void path investigated and CLOSED as unreachable**: the transition table (`foundation/src/enums.rs`) only allows `Active→Voided` — a completed (paid, points-awarded) sale can never be voided, only refunded. That sweep did surface a real race in `void_sale`: the Active pre-check read outside the transaction and the UPDATE had no status predicate, so a concurrent finalize could be overwritten completed→voided; fixed as a compare-and-set (`AND status = 'active'`, explicit rollback on conflict).
- ~~**LOY-04** — Tier updates accept invalid business values~~ — **VERIFIED FIXED 2026-08-31** (registry stale; re-recorded after an earlier close note was lost to a foreign tree revert): `update_tier` runs `validate_tier_config` — empty names, negative thresholds, non-positive `points_per_unit`, non-finite/non-positive multipliers and non-hex colours are rejected, plus unique-threshold and zero-tier invariants; pinned by `update_tier_rejects_invalid_values`.
- **LOY-05** — Load failures are silently presented as stale or empty data

---

## Reporting (`03-reporting-module.md` — REMEDIATED IN PART)

**Status:** security boundary and limit validation implemented; **REP-02/04/05(erase)/06 closed; remaining: REP-03, REP-05 rewrite-half (snapshot-column design), pie visual semantics + cloud email parity follow-ups**.

Key open items:
- ~~**REP-02** — Revenue UI combines different currencies into one displayed total~~ — **VERIFIED FIXED 2026-08-30**: per-currency summing in `ui/src/features/reports/revenueTotals.ts` + DashboardScreen/SalesReportScreen tests.
- ~~**REP-04** — Report queries do not show explicit refund/void/net-sales treatment~~ — **CLOSED 2026-08-30** (core `35d8bec4`; UI half landed inside foreign commit `98300bca` — content verified intact, attribution recorded in the journal): refunds never mutate the sale row, so revenue counted refunded sales at full value and the refund ledger was invisible everywhere. Daily/weekly/monthly revenue now aggregate sales and refunds via CTEs joined FULL OUTER on (period, currency) — each row carries `refund_minor` (attributed to the REFUND's own period) + `net_revenue_minor`; refund-only periods produce a row instead of dropping the refund. `refunds_summary` (per-currency count + totals) added in core — **no IPC/UI consumer yet** (intended for a future refunds panel; do not leave it orphaned). Voids were already surfaced (`voided_sales_summary`); net-sales semantics are now explicit in the row fields. SalesReportScreen shows Refunds + Net Revenue rows when the period has refunds.
- ~~**REP-06** — cross-currency SUMs in the remaining report queries~~ — **CLOSED 2026-08-31** (core `38b456bd`, UI `3f9ced5c`): `top_products`, `hourly_heatmap`, `category_breakdown`, `payment_method_breakdown` and `voided_sales_summary` summed minor units across currencies into one number (the REP-02 class below the revenue trends). All five now GROUP BY currency with a `currency` field on every row; category percentages normalize WITHIN each currency; voided summary returns one row per currency; heatmap cells aggregate currency rows (orders sum, intensity tracks the display currency, labels list every amount); the refunds analytics card + its CSV are per-currency. **Follow-ups recorded:** (a) the category PIE still compares slice areas across currencies — visual-semantics decision needed (per-currency pies vs currency filter); (b) cloud `email_pg.rs` mirrors the old single-currency shapes AND lacks REP-04 refund netting — parity slice.
- ~~**REP-05** — Current product/category joins can erase or rewrite historical sales attribution~~ — **ERASE HALF CLOSED 2026-08-31 (`cd4bdaa8`)**: `top_products`/`category_breakdown` INNER JOINed the mutable products table — deleting a product silently erased its historical sales from both reports (totals stopped reconciling; the category pie inflated surviving slices). Both LEFT JOIN now: deleted products keep revenue under their stored sku and bucket into Uncategorised. **Rewrite half remains open as a design item**: renames/category moves still retroactively change historical labels because `sale_lines` stores only `sku` — fixing it needs sale-line snapshot columns (name/category at sale time), a backfill for legacy rows, and cloud-sync parity; deliberately not smuggled into this slice.
- **REP-03** — Date boundaries have no store-timezone contract or input validation
- Also: stale-request races, unbounded custom-report results, CSV escaping

---

## Currency (`04-currency-module.md` — PARTIALLY REMEDIATED)

**Status:** IPC fixed-point contract aligned; **settlement, command scoping, and remaining UX/validation findings require follow-up**.

Key open items:
- ~~**CUR-02** — PaymentModal displays converted currency but settles the base currency amount~~ — **VERIFIED FIXED 2026-08-30**: tender snapshot (base currency / base total / fixed-point rate) flows PaymentModal → `complete_sale*` args → `Sale` → SQLite (`20260821_tender_currency.sql`) and is now also persisted to the cloud (`pg::create_sale`, commit bc8bb29c — the PG INSERT had silently dropped all five tender/tip/service columns that `pg::get_sale` reads).
- ~~**CUR-04** — PaymentModal chooses the first matching rate without selecting the effective historical rate~~ — **CLOSED 2026-08-31 (`3ff5db6f`)**: the session path already asks the backend for the latest rate effective as-of today (`get_latest_exchange_rate_scoped` → `effective_date <= as_of ORDER BY effective_date DESC` — correct); settlement itself is snapshot-based by design (tender stores the payment-time fixed-point rate, CUR-02). The residual was the no-session fallback: `list_exchange_rates` ordered only by pair, so `find()` could surface the oldest-inserted rate — now newest-effective-first within each pair (Red-first backfill test).
- ~~**CUR-05** — Exchange-rate input and effective-date validation is incomplete~~ — **VERIFIED FIXED 2026-08-30** (see Currency section: validators on both clients + UI gating + `type="date"`; registry entry was stale).
- ~~**CUR-06** — Default-currency command scope~~ — **CLOSED 2026-08-30**, commit `aa1f831f` (backend scoping existed; `ExchangeRateScreen` was the un-migrated UI consumer — now routes through `*_scoped` APIs).
- ~~**CUR-10** — missing delete confirmation~~ — **VERIFIED FIXED 2026-08-30** (`ConfirmDialog` before delete, pinned by tests). **CUR-09/CUR-11** remain open (locale/theme gaps; bounded-rate API + e2e coverage).
- Currency exponent-aware settlement rounding and a lossless string/decimal IPC representation for values beyond JavaScript's safe integer range

---

## Staff (`06-staff-module.md` — PARTIALLY REMEDIATED)

**Status:** security-critical staff IPC paths closed; **residuals documented**.

Key residual items:
- **STAFF-13 (partially)** — Security-focused command tests and wiring tests cover session binding, two-store identity, role hierarchy, PIN rotation/invalidation, rate limits… (remaining coverage gaps documented in the deleted report)
- ~~Deactivate/Restore flow: … backend does not prevent self/last-owner deactivation~~ — **backend half VERIFIED FIXED 2026-08-30** (registry stale): `enforce_role_assignment_policy` on BOTH clients' scoped commands blocks self-role change, self-deactivation (STAFF-10), and deactivate/demote of the last active Owner (STAFF-02), plus Owner-only promotion; legacy unscoped staff commands are hard-disabled. Branch-pinning tests added (`d45d1119`: each guard exercised in isolation with exact-message asserts — the pre-existing last-owner test was satisfied by either branch). **Remaining (UI, open):** no confirmation dialog, no per-row pending state. **Coverage gap noted:** the tablet's duplicated policy copy has NO command-level tests (tablet test infra lacks a scoped-state helper — worth a small harness slice).

---

## Loading States (`23-loading-states.md` — AUDITED)

**Status:** **ALL CLOSED — verified fixed 2026-08-31** against current code; a dedicated guard test (`__tests__/loadingStateCompliance.test.tsx`) now enforces the patterns.

- ~~**LOAD-01** — Two separate Skeleton components~~ — **VERIFIED FIXED**: single canonical `frontend/shared/Skeleton.tsx`; `components/Skeleton.tsx` is a documented 9-line compatibility re-export so 40+ importers share one source of truth.
- ~~**LOAD-02** — Load failures silently become an empty screen~~ — **VERIFIED FIXED**: sampled failure paths (ShiftBar, StockCountDetail, KdsProductPickerModal, CustomerManagement) all surface localized error toasts or error states; compliance test guards the pattern.
- ~~**LOAD-03** — Demo-data fallbacks mask production failures~~ — **VERIFIED FIXED**: no demo-data path remains in any data-loading screen (analytics card states it explicitly); remaining "demo" hits are the intentional DesignSystem showcase pages.
- ~~**LOAD-04** — Initial-load vs refresh semantics (KdsScreen)~~ — **VERIFIED FIXED**: `KdsScreen` gates the skeleton on `initialLoading` only; refreshes update the live board in place.
- ~~**LOAD-05** — Progress not announced~~ — **VERIFIED FIXED**: shared `LoadingStatus` (role=status, localized label) wraps decorative skeletons; 11 screen usages + compliance test.

---

## Topology — Rust (`30-topology-rust.md` — ⚠️ 8 FINDINGS, OPEN)

**Status:** **ALL CLOSED — verified fixed 2026-08-31** (TOP-01→TOP-08).

- ~~**TOP-01** — Raw control bytes in test string literals~~ — **VERIFIED FIXED**: no control-byte literals remain; `topology_setting_key` now REJECTS control chars (dedicated tests: `..._control_chars_rejected`, `..._rejects_path_injection`).
- ~~**TOP-02** — Setting-key constant duplicated across three modules~~ — **VERIFIED FIXED**: single home in `topology/model.rs` (`TOPOLOGY_SETTING_KEY`, `TOPOLOGY_RUNTIME_SETTING_KEY`, `TOPOLOGY_APPLY_*`, `TOPOLOGY_SCHEMA_VERSION`), consumed via `pub(crate)`.
- ~~**TOP-03** — Duplicated paragraph in `load_topology_data` doc~~ — **VERIFIED FIXED**: doc is a single coherent explanation (load-side raw-port faithfulness + healing rationale), pinned by `load_topology_data_preserves_raw_legacy_null_ports`.
- ~~**TOP-04** — `save_topology_data` duplicated structural validation~~ — **VERIFIED FIXED**: save calls the shared `validate_topology_structure` (persistence.rs:726); the comment records the drifted inline copy's removal.
- ~~**TOP-05** — Legacy typed API production-dead~~ — **VERIFIED FIXED**: `save_topology_data`/`load_topology_data` are now `#[cfg(test)]`-gated test-compat helpers; production flows through `apply_topology_diff` only.
- ~~**TOP-06** — Missing doc comments~~ — **VERIFIED FIXED**: sampled items (`default_direction`, `topology_apply_request_key`, `persist_topology_recovery`) all carry doc comments.
- ~~**TOP-07** — O(n²) scans in validation core~~ — **VERIFIED FIXED**: membership checks use prebuilt sets (`workspace_id_set.contains`, `warehouse_ids` via set lookups).
- ~~**TOP-08** — File-size watch items~~ — **NO ACTION**: `persistence.rs` 786 lines (under the 1,000 cap); test files within the module's cited guideline. Watch item only.

---

## Money — Frontend (`32-money-frontend.md` — FULLY REMEDIATED)

**Status:** FRONTEND-01/02/03/04 all closed (04 found + fixed during the FRONTEND-03 sweep, 2026-08-30).

- **FRONTEND-01** — PaymentModal charge-amount row inflates by base exponent (P1, FIXED)
- **FRONTEND-02** — usePosState silently mixes currencies in the subtotal (P1, FIXED)
- ~~**FRONTEND-03** — IPC boundary drops line currency (P2, **DEFERRED to Phase 5 — open, needs backend change**)~~ — **CLOSED 2026-08-30**, commit `fc8eae22`: `AddLineArgs.unit_price_currency` (optional, wire-compatible) added on desktop + tablet; commands build the line in the wire currency so `Cart::add_line`'s (previously dead) mismatch check rejects cross-currency lines; invalid ISO codes fail closed. PaymentModal sends `line.unit_price.currency` on both sale paths. Pinned by tablet e2e (EUR line into USD cart → Err) + desktop serde-shape/helper tests + UI contract tests. Follow-up also **CLOSED 2026-08-30**, commit `4439cfa3`: `CartLineData.unit_price_currency` in `complete_sale_with_resolved_shortfalls_scoped` (both clients) — same helper pattern, same fail-closed parse; PaymentModal's shortfall-dialog mapping sends the line currency, dialog passthrough pinned by test.
- ~~**FRONTEND-04** (P2, found 2026-08-30 during the FRONTEND-03 sweep) — multi-currency charge + stock shortfall settles the second command in the WRONG currency~~ — **CLOSED 2026-08-30**, commit `0e5e8bf9`. Semantics decision: the retry must settle in the SAME currency the first command used (charge currency) — it is a retry of the same sale. Fix (UI-only; the Rust struct already accepted all five CUR-02 fields): CUR-02 tender metadata lifted into a shared `tenderSnapshot` memo used by the QRIS path, main path, and shortfall dialog; dialog now receives `lineItemsInCartCurrency` + `cartCurrency` + `effectiveTotalInCartCurrency` + `tenderedMinorInCartCurrency` and forwards tip/service/base fields into the retry args. Pinned by a multi-currency e2e (USD cart → IDR charge at 16500: retry payload currency IDR, converted line amounts, baseCurrency/baseTotalMinor/tenderRateMillionths) + single-currency tip/service pin (previously the retry recorded tip=0/service=0 even without multi-currency).

---

## Currency — Exchange & Settlement (`34-currency-exchange.md` — MOSTLY REMEDIATED)

**Status:** CUR-02/03/04/05/06/08/10 closed; **CUR-09, CUR-11 remain open** (locale/theme gaps; bounded-rate API + e2e coverage).

- ~~**CUR-02** (P0) — PaymentModal displays converted currency but settles the base currency amount (multi-currency settlement)~~ — **CLOSED 2026-08-30** (see Currency section: local snapshot verified in code; cloud persistence gap in `pg::create_sale` fixed, commit bc8bb29c, pinned by the PG roundtrip test).
- ~~**CUR-05** (remaining) — exchange-rate input / effective-date validation residuals~~ — **VERIFIED FIXED 2026-08-30** (registry was stale): `validate_create_rate_args` on desktop AND tablet (shared by legacy + scoped paths) rejects non-positive rates, same-currency pairs, non-ISO-4217 codes, and malformed `YYYY-MM-DD` effective dates; UI side `ExchangeRateScreen` gates Save on the same conditions incl. the millionths round-trip (`Number.isSafeInteger`), and the date field is `type="date"` (browser-enforced format). Pinned by `exchange_rates_tests.rs` (zero/negative, same-pair, non-ISO, malformed-date, valid-input).
- ~~**CUR-06** — default-currency command scope~~ — **CLOSED 2026-08-30**, commit `aa1f831f`: backend scoped variants (`get/set_default_currency_scoped`, rate commands) already enforced `SETTINGS_READ`/`SETTINGS_EDIT` + store resolution with ISO validation on set; the residual was the UI — `ExchangeRateScreen` still called the legacy global-DB APIs for list/create/delete. Now routes through the `*_scoped` wrappers whenever a workspace session exists. Pinned by 3 new tests (token passthrough + legacy-not-called).
- ~~**CUR-10** — missing delete confirmation~~ — **VERIFIED FIXED 2026-08-30**: `ExchangeRateScreen` renders a `ConfirmDialog` (`currency-delete-confirm`, danger variant) before `deleteExchangeRate`; pinned by the delete tests.
- **CUR-09** — locale/theme gaps (unverified; low value, left open)
- **CUR-11** — bounded/latest-rate APIs + e2e coverage (`get_latest_exchange_rate_scoped` exists since CUR-04; bounded list + Playwright coverage still open)

---

## Refund guards — `create_refund` (rust-auditor COR-25/COR-26 — CLOSED 2026-08-30)

- ~~**COR-25** (MEDIUM) — over-refund guard ran outside the transaction and read the cumulative refunded SUM with `.unwrap_or(0)`: a read error read as "zero refunds" and the money guard failed OPEN~~ — fixed: guard inside the tx, SUM errors propagate (`crates/oz-core/src/db/refunds.rs`, commit 8f01a5d0; regression test `over_refund_guard_fails_closed_when_cumulative_sum_unreadable`).
- ~~**COR-26** (LOW) — refund currency never compared to the sale currency~~ — fixed: `create_refund` rejects a mismatch with `CoreError::CurrencyMismatch` (commit a53feaea; regression test `create_refund_rejects_currency_mismatch`).

## PG integration harness — silently-skipped tests (CLOSED 2026-08-30)

`throwaway_test_pool` built DB names from UUID `Display` (hyphens) inside an
unquoted `CREATE DATABASE` identifier → server syntax error → every
throwaway-DB PG test (REST roundtrip, RLS non-owner, concurrent adjust,
sync-store) printed "skipped" and reported PASS — cloud money-path coverage
was silently zero. Fixed with `.simple()` hex names + probe connections
retargeted to the throwaway DB (commit a022b4fb). **Verified live: oz-api
198/198 and oz-cloud-server 224/224 with ZERO skips.** Residual risk: on
machines/CI without PG the skip is still quiet (PASS) by design.

---

## Topology — Editor UI (`31-topology-editor-ui.md` — ✅ ALL CLOSED 2026-08-26)

**Status:** **all 7 findings repaired in one commit** (TOP-UI-01→TOP-UI-07). Audit of
`ui/src/features/stores/` (33 production files, ~12k lines) + docs. No blocking
findings; verdict was "solid code, stale docs".

Closed items:
- **TOP-UI-01** — ADR #34 pairing table listed 8 rows; shared contract (`topologySemantics.json`,
  frontend + Rust `include_str!`) implements 7. `location-out → operation-in` removed from the
  ADR table with an explanatory note; parent ADR #34 §Slice 1 corrected to match.
- **TOP-UI-02** — ADR #34 audit stamp (09-08-26) predated 12-08-26 sections; new sections
  re-verified during this audit, stamp refreshed to 26-08-26.
- **TOP-UI-03** — ADR #34 cited `topologyCard.ts` as the pairing-table home and row order that
  drifted from the JSON; doc now references `topologySemantics.json` and matches row order.
- **TOP-UI-04** — `docs/api-reference.md` missing `can_save_topology` command; row added.
- **TOP-UI-05** — `topology.rs` module doc said "four #[tauri::command]" but only 3 exist
  (the 4th is a startup daemon); comment corrected to "three".
- **TOP-UI-06** — 4 pre-existing `topologyNodeCard.test.tsx` failures (validation text renders
  twice: tooltip + SR-only span; dismiss button hidden in the tooltip portal); tests now query
  `getAllByText` / `hidden: true`.
- **TOP-UI-07** — `docs/multi_pos_one_location_support.md` cited `topologyEditor.tsx`; actual
  file is `NodeTopologyEditor.tsx`; path corrected.

---

## How to close these

Each finding's original remediation guidance lives in git history under
`audit/<NN>-<slug>.md` (the file was consolidated into this document; the
per-finding fix descriptions, affected files, and commit lists remain
available via `git log` on the deleted path). Re-open a finding here, fix
it, then flip its status in this file.
