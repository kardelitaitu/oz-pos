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

**Status:** CRM-01 resolved (session-scoped customer-management path); **CRM-02–CRM-11 remain open**.

Key open items:
- **CRM-02** — Customer listing does not enforce the view permission
- **CRM-03** — Load failures are silently rendered as an empty customer database
- **CRM-04** — Delete is immediate and delete failures are invisible
- **CRM-05** — "Purchase history" is documented but not exposed as customer history
- CRM-06–CRM-11 — remaining findings (see deleted report / git history `audit/01-crm-module.md`)

---

## Loyalty (`02-loyalty-module.md` — AUDITED)

**Status:** LOY-01 remediated; **remaining findings require follow-up**.

Key open items:
- ~~**LOY-02** — Earning points is not idempotent by sale~~ — **VERIFIED FIXED 2026-08-30**: migration 128 enforces a unique earn/redeem projection index (`crates/oz-core/src/db/loyalty_tests.rs:556`).
- **LOY-03** — No refund or void compensation path for earned points
- **LOY-04** — Tier updates accept invalid business values
- **LOY-05** — Load failures are silently presented as stale or empty data

---

## Reporting (`03-reporting-module.md` — REMEDIATED IN PART)

**Status:** security boundary and limit validation implemented; **financial/reporting UX findings remain open**.

Key open items:
- ~~**REP-02** — Revenue UI combines different currencies into one displayed total~~ — **VERIFIED FIXED 2026-08-30**: per-currency summing in `ui/src/features/reports/revenueTotals.ts` + DashboardScreen/SalesReportScreen tests.
- **REP-03** — Date boundaries have no store-timezone contract or input validation
- **REP-04** — Report queries do not show explicit refund/void/net-sales treatment
- **REP-05** — Current product/category joins can erase or rewrite historical sales attribution
- Also: stale-request races, unbounded custom-report results, incomplete refund/void semantics in reporting queries, CSV escaping

---

## Currency (`04-currency-module.md` — PARTIALLY REMEDIATED)

**Status:** IPC fixed-point contract aligned; **settlement, command scoping, and remaining UX/validation findings require follow-up**.

Key open items:
- ~~**CUR-02** — PaymentModal displays converted currency but settles the base currency amount~~ — **VERIFIED FIXED 2026-08-30**: tender snapshot (base currency / base total / fixed-point rate) flows PaymentModal → `complete_sale*` args → `Sale` → SQLite (`20260821_tender_currency.sql`) and is now also persisted to the cloud (`pg::create_sale`, commit bc8bb29c — the PG INSERT had silently dropped all five tender/tip/service columns that `pg::get_sale` reads).
- **CUR-04** — PaymentModal chooses the first matching rate without selecting the effective historical rate — **partially addressed**: `PaymentModal.tsx:266` now asks the backend for the latest rate when a session store is active; historical-effective-rate selection at settlement still open.
- **CUR-05** — Exchange-rate input and effective-date validation is incomplete
- **CUR-06** — Default-currency command scope
- **CUR-09/10/11** — Locale/theme gaps, missing delete confirmation, bounded/latest-rate APIs, end-to-end coverage
- Currency exponent-aware settlement rounding and a lossless string/decimal IPC representation for values beyond JavaScript's safe integer range

---

## Staff (`06-staff-module.md` — PARTIALLY REMEDIATED)

**Status:** security-critical staff IPC paths closed; **residuals documented**.

Key residual items:
- **STAFF-13 (partially)** — Security-focused command tests and wiring tests cover session binding, two-store identity, role hierarchy, PIN rotation/invalidation, rate limits… (remaining coverage gaps documented in the deleted report)
- Deactivate/Restore flow: no confirmation dialog, no per-row pending state, backend does not prevent self/last-owner deactivation — recommendation: confirm with staff name + consequences, disable while pending, prevent self/last-owner deactivation in backend

---

## Loading States (`23-loading-states.md` — AUDITED)

**Status:** **cross-screen loading and failure-state findings require remediation**.

Key open items:
- **LOAD-01** — Two separate Skeleton components create visual and behavioral drift
- **LOAD-02** — Several load failures are silently converted into an apparently empty screen
- **LOAD-03** — Demo-data fallbacks can mask a production load failure
- **LOAD-04** — Loading semantics inconsistent for initial load versus refresh (e.g. `KdsScreen` skeleton, then direct board updates without a refreshing state)
- **LOAD-05** — Custom skeletons and plain loading text do not consistently announce progress

---

## Topology — Rust (`30-topology-rust.md` — ⚠️ 8 FINDINGS, OPEN)

**Status:** **all 8 findings open** (TOP-01→TOP-08, all P2/P3; no P0/P1). No production code changed — audit only, fixes pending approval.

Key items:
- **TOP-01** — Raw control bytes (NUL/SOH) embedded in test string literals
- **TOP-02** — Runtime setting-key constant duplicated across three modules
- **TOP-03** — Duplicated paragraph in `load_topology_data` doc comment
- **TOP-04** — `save_topology_data` duplicates structural validation with drift risk
- TOP-05–TOP-08 — remaining hygiene/consistency fixes recommended before heavy expansion

---

## Money — Frontend (`32-money-frontend.md` — PARTIALLY REMEDIATED)

**Status:** 3 findings closed, **1 open (deferred to Phase 5)**.

- **FRONTEND-01** — PaymentModal charge-amount row inflates by base exponent (P1, FIXED)
- **FRONTEND-02** — usePosState silently mixes currencies in the subtotal (P1, FIXED)
- **FRONTEND-03** — IPC boundary drops line currency (P2, **DEFERRED to Phase 5 — open, needs backend change**)

---

## Currency — Exchange & Settlement (`34-currency-exchange.md` — PARTIALLY REMEDIATED)

**Status:** CUR-03, CUR-04, CUR-08 closed (P0/P1/P2); **CUR-02, CUR-05-remaining, CUR-06, CUR-09, CUR-10, CUR-11 remain open**.

- ~~**CUR-02** (P0) — PaymentModal displays converted currency but settles the base currency amount (multi-currency settlement)~~ — **CLOSED 2026-08-30** (see Currency section: local snapshot verified in code; cloud persistence gap in `pg::create_sale` fixed, commit bc8bb29c, pinned by the PG roundtrip test).
- **CUR-05** (remaining) — exchange-rate input / effective-date validation residuals
- **CUR-06** — default-currency command scope
- **CUR-09/10/11** — locale/theme gaps, missing delete confirmation, bounded/latest-rate APIs

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
