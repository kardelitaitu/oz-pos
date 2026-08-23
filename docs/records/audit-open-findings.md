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
- **LOY-02** — Earning points is not idempotent by sale
- **LOY-03** — No refund or void compensation path for earned points
- **LOY-04** — Tier updates accept invalid business values
- **LOY-05** — Load failures are silently presented as stale or empty data

---

## Reporting (`03-reporting-module.md` — REMEDIATED IN PART)

**Status:** security boundary and limit validation implemented; **financial/reporting UX findings remain open**.

Key open items:
- **REP-02** — Revenue UI combines different currencies into one displayed total
- **REP-03** — Date boundaries have no store-timezone contract or input validation
- **REP-04** — Report queries do not show explicit refund/void/net-sales treatment
- **REP-05** — Current product/category joins can erase or rewrite historical sales attribution
- Also: stale-request races, unbounded custom-report results, incomplete refund/void semantics in reporting queries, CSV escaping

---

## Currency (`04-currency-module.md` — PARTIALLY REMEDIATED)

**Status:** IPC fixed-point contract aligned; **settlement, command scoping, and remaining UX/validation findings require follow-up**.

Key open items:
- **CUR-02** — PaymentModal displays converted currency but settles the base currency amount
- **CUR-04** — PaymentModal chooses the first matching rate without selecting the effective historical rate
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

- **CUR-02** (P0) — PaymentModal displays converted currency but settles the base currency amount (multi-currency settlement)
- **CUR-05** (remaining) — exchange-rate input / effective-date validation residuals
- **CUR-06** — default-currency command scope
- **CUR-09/10/11** — locale/theme gaps, missing delete confirmation, bounded/latest-rate APIs

---

## How to close these

Each finding's original remediation guidance lives in git history under
`audit/<NN>-<slug>.md` (the file was consolidated into this document; the
per-finding fix descriptions, affected files, and commit lists remain
available via `git log` on the deleted path). Re-open a finding here, fix
it, then flip its status in this file.
