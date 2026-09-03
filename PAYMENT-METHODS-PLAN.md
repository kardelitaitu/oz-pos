# Payment Methods per Workspace — Planning Notes

> **Status:** DISCUSSION NOTES — not yet a spec. Next step tomorrow: answer the open questions, then draft `docs/specs/_active/` package (spec.yaml + plan.md + validation.md).
> **Date:** 2026-09-03 · Session with DSH (DeepSeek Harness agent)

---

## 0. Context — how we got here

Three topics were reviewed today, in order:

1. **Role contract (RBAC)** — traced end-to-end. Verdict: **already solid, no work needed.**
   - `platform/core/src/rbac.rs`: `Role` = name + JSON permission list; 84 compile-time permission constants (`domain:action`).
   - Wildcard resolver: `*` → everything, `sales:*` → domain, `sales:void` → exact. Empty/malformed → **deny all** (fail-closed).
   - Presets (`rbac_presets.rs`): Owner=`*` · Admin=all minus billing/ownership/`staff:delete` · Manager=broad but no `staff:manage_roles` · Staff=checkout-only · Auditor=read-only · Custom=empty.
   - Enforcement at every command via `require_permission_for_session` (`apps/desktop-client/src/commands/authz.rs`) + scope check (store/workspace assignment, ADR #35).
   - Bidirectional inventory tests anchor registry ↔ presets ↔ `ALL_ENFORCED`.

2. **Subscription contract** — traced end-to-end. Verdict: **already solid, no work needed.**
   - `crates/oz-core/src/subscription.rs`: tiers Free/Plus/Pro/Premium/Enterprise (+ deprecated `OneTime` for DB back-compat).
   - Quotas: `max_stores` (1/1/2/5/∞), `max_pos_instances` (1/2/5/∞/∞), `max_warehouses` (1/2/3/∞/∞), `max_staff_users` (1/5/20/50/∞), `sales_history_days` (90/365/5y/∞/∞).
   - Features: QRIS (Plus+), cloud sync (Plus+), Stripe (Pro+), analytics (Pro+), KDS (Pro+), loyalty + Lua engine (Premium+), regional zones (Enterprise).
   - `effective_tier()`: canceled → Free; within `offline_grace_days` → paid tier; out of grace → Free. Clock-rollback detection via ledger MAX(created_at) vs wall clock (30s tolerance). Row is **RSA-2048-signed** by the license server.
   - Enforced at creation points: staff (staff.rs), warehouses (inventory.rs), registers (topology), workspace types, history capping (history.rs).

3. **TOML configuration idea** (`staff do what: true/false`, `max_terminal = 3`) — verdict: **shelved.**
   - Roles: model is already TOML-shaped; recommendation if ever wanted = **import/export + first-boot seed**, DB stays source of truth.
   - Subscription: limits are server-signed entitlements; TOML could only ever be **restrict-only** local caps (`min(signed, toml)`). Niche. No pain today → no work.

---

## 1. Today's main topic: workspace payment-method configuration

Goal: per-workspace-type enable/disable of payment methods in the payment modal (retail `store-pos` vs resto `restaurant-pos`), plus a Midtrans ID home for dynamic QRIS, plus a retail-only "pay later" (piutang) method.

### 1.1 Current-state map (verified facts)

| # | Fact | Location | Implication |
|---|---|---|---|
| 1 | Methods are **hardcoded** in the modal: `['cash','card','qris','credit']` + free-text "other" + `open_bill`; identical for retail and resto | `ui/src/features/sales/PaymentModal.tsx:1443`, type at line 38 | The core ask genuinely doesn't exist yet |
| 2 | `payments.method` is a **free string** column | `crates/oz-core/src/payment.rs`; migration `022_payments_table.sql` | New method values need **no schema change** |
| 3 | `credit` already exists but means **customer tab** (name + later settlement), NOT credit card. Settlement SQL filters `method = 'credit'` | PaymentModal:1497; `apps/desktop-client/src/commands/settings.rs:359,396` | ⚠️ Naming collision: "credit card" needs a different value (proposal: keep `card` for EDC, or `card_credit`/`card_debit` split — open question #2) |
| 4 | Subscription caps already gate method visibility: `useSubscription().caps` → QRIS locked teaser when `!caps.supportsQris` | PaymentModal:1592, SetupWizard:695 | Config must **intersect** with signed entitlements — pattern already exists |
| 5 | `payment_gateways` table EXISTS (migration `20260825_payment_infra.sql`) with stub CRUD: `name` (stripe/square/midtrans/paddle), `is_active`, `config_json` | `crates/oz-core/src/db/payment_gateways.rs` | "Midtrans ID" has a designated home; stubs must be implemented; audit note: **encrypt keys at rest** (CRY-1 tie-in) |
| 6 | Gateway plumbing pre-built on `Payment`: `gateway_reference/status/response`, `idempotency_key` (PAY-2 drop point lives in oz-payment drivers) | `payment.rs`; migration `027_payment_gateway_fields.sql` | Midtrans integration slots in without schema changes |
| 7 | `MIDTRANS_SERVER_KEY` env validation exists (`Mid-server-`/`SB-Mid-server-` prefixes) | `crates/oz-core/src/config_validator.rs:162` | Env-var path works today; per-store config is the next step |
| 8 | `type_key` (`store-pos`/`restaurant-pos`) flows through session + WorkspaceContext; Settings page already has separate `store-pos` and `restaurant-pos` sections | `SettingsPage.tsx:806-813`, `SettingsNavTree.tsx:114-125` | Config UI has a natural home; retail-only gating trivial |
| 9 | Settings storage = `settings` table (key, JSON value) per store DB | `db/popularity.rs:574,630` | Phase 1 rides this — **no migration needed** |
| 10 | **No piutang/receivable concept exists** anywhere in the codebase | (grep: only an unrelated comment) | "Pay later" is greenfield — needs real design (§1.5) |

### 1.2 The configuration shape (Phase 1)

One settings key per workspace type, JSON value (TOML-able later — same shape):

```toml
# Conceptual shape — stored under settings key "payment_methods.<type_key>"
[payment_methods.store-pos]        # retail
cash          = true
card          = true               # EDC terminal flow (hardware exists today)
qris_manual   = true               # printed QRIS at counter; cashier confirms manually on phone
bank_transfer = false              # manual check
qris          = false              # dynamic (Midtrans) — Phase 2
ewallet       = false              # GoPay/OVO/DANA/ShopeePay via Midtrans — Phase 2
pay_later     = false              # retail-only piutang — Phase 3

[payment_methods.restaurant-pos]   # resto
cash          = true
qris_manual   = true
open_bill     = true               # resto-only, already exists
credit        = true               # customer tab, already exists
card          = true
# no pay_later — restaurant-pos never gets it
```

**Safety rules (non-negotiable):**

1. Effective method list = **`config ∩ subscription entitlements ∩ workspace type`**.
2. UI hides disabled methods, but the **backend checkout command re-validates** against the effective config — never trust the UI.
3. Malformed/unparseable config JSON → **fail closed to current defaults** (cash/card always available as floor).
4. Dynamic methods additionally require their signed entitlement (`caps.supportsQris` etc.).

Settings UI: "Payment Methods" section inside the existing `store-pos` / `restaurant-pos` pages of the Tauri Settings page (master–detail + Save — the existing pattern).

### 1.3 Manual vs dynamic QRIS (the key distinction)

| | `qris_manual` | `qris` (dynamic) |
|---|---|---|
| How | Static printed QRIS at the counter | Midtrans SNAP dynamic QR with exact amount |
| Verification | Cashier sees money arrive on their phone, taps "confirmed" | App polls Midtrans transaction status; auto-confirmed |
| Needs | Nothing — Phase 1 | Midtrans server key (encrypted), network, `supports_qris` (Plus+) |
| Record | `method="qris_manual"`, settled immediately | `method="qris"`, `gateway_reference` + status from Midtrans, `idempotency_key` |

Phase 1 ships `qris_manual` + `bank_transfer` with **zero gateway work** — immediate value.

### 1.4 Midtrans (Phase 2)

- Implement the stubbed `payment_gateways` CRUD: upsert/list/get; `config_json` **encrypted at rest**; per-tenant row with `is_active`.
- Midtrans SNAP client in Rust (desktop client is the backend): create transaction → show QR → poll status → write `gateway_reference`/`gateway_status` on the payment.
- `idempotency_key` prevents double-charge on retry (field already in schema, waiting).

### 1.5 Pay later / piutang (Phase 3 — retail only)

- New method value `pay_later` + new `receivables` table:
  `sale_id, customer_id, amount, due_date, status(open|partial|paid), settled_at` — real migration (SQLite registry + PG drift-guard rules apply).
- Reminder surfaces (pick): due-soon widget on Daily Dashboard; line in reports; optional toast on POS open.
- ⚠️ **Recommendation against "mark it as cash" in the DB:** piutang recorded as `method='cash'` silently corrupts cash-drawer reconciliation and "cash in drawer" reports, and loses "how much do customers owe me". Instead: `method='pay_later'` + optional **reporting toggle** "count pay-later as cash-equivalent" if grouping is wanted. Same completion semantics, no corrupted drawer math.

---

## 2. Open questions (answer tomorrow)

1. **Piutang in reports** — separate `pay_later` method (+ optional "count as cash" report toggle) — recommended — or strictly inside the cash total?
2. **Debit vs credit card** — one `card` button (today's EDC flow) or two separate buttons? Matters only if deposits reconcile differently.
3. **Config scope** — per workspace *type* first (one config for all retail, one for all resto), per-*instance* override later? (Precedent: `terminal_override`.)
4. **E-wallet scope** — Midtrans umbrella only, or each provider as an individually toggleable button?
5. **Pay-later permission** — new `payments:pay_later` permission key (clean; ripples through registry/presets/`ALL_ENFORCED`) or ride existing `payments:cash`?

---

## 3. Decisions so far

| Decision | Status |
|---|---|
| Role/subscription contracts need no rework | ✅ agreed (today) |
| TOML auth/quota configs shelved | ✅ agreed (today) |
| Payment-method config keyed by workspace type in `settings` table (Phase 1, no migration) | leaning yes — confirm tomorrow |
| `qris_manual` + `bank_transfer` as Phase 1 manual-check methods | proposed |
| Midtrans via existing `payment_gateways` table + encryption (Phase 2) | proposed |
| Piutang = `pay_later` method + `receivables` table, NOT marked as cash (Phase 3, retail-only) | proposed — user to confirm |
| Backend re-validates method list (fail-closed), UI hiding is cosmetic | proposed |

## 4. Tomorrow's next steps

1. Answer open questions §2.
2. If green-lit: draft spec package `docs/specs/_active/<next-id>-workspace-payment-methods/` (spec.yaml + plan.md + validation.md per house format; see `0043-architecture-boundary-checker` as the example).
3. Phase 1 implementation order would be: settings key + load/validate module → backend checkout validation → PaymentModal reads config (delete hardcoded array) → Settings UI section → tests (unit + `PaymentModal*.test.tsx` + contract tests).

---

*Raw discussion trace: sessions with DSH on 2026-09-03. Code references verified at branch `0.0.35` (HEAD at time of writing: `8beda436`).*
