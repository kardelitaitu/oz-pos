# Subscription Tier Finalization

> **Status:** DRAFT — D1 (lineup) and D2 (pricing model) are resolved;
> D3–D9 still need decisions.
> **Date:** 2026-08-17
> **Why this exists:** the repo carried several overlapping (and mutually
> contradictory) tier definitions — the business plan, ADR #5, the oz-core
> entitlement enum, and the live website + Paddle implementation. The lineup
> and pricing model are now decided (see D1 + D2); the remaining work is
> filling in quotas and feature gates per tier, confirming the exact price
> points, then aligning every file with this document.

---

## 1. The decided lineup (D1 — RESOLVED 2026-08-17)

**Five tiers: `Free` · `Plus` · `Pro` · `Premium` · `Enterprise`**

- **Free** is a **free-forever plan with 1 workspace only** — it *replaces*
  the old 90-day trial card. No checkout, no payment method.
- **Plus** is a new entry-level paid tier, inserted between Free and Pro.
- **Pro** ($19/mo) and **Premium** ($49/mo) keep their existing Paddle prices.
- **Enterprise** stays bespoke (no Paddle price, contact-sales path).
- **Pricing model (D2):** every paid tier is sold in **USD and IDR display** ×
  **monthly and yearly**; yearly prices are rounded — **USD ≈17% off at every
  tier** ($50/$100/$250), **IDR keeps an increasing 15–25% ladder**.

What each old model contributed (and where it lands now):

| Old model | Old tiers | Fate under the 5-tier lineup |
| :--- | :--- | :--- |
| BUSINESS_PLAN.md | 1-Time / Standard / Pro / Enterprise | **1-Time, Standard: retired.** Their features fold into the Plus/Pro ladder as needed. |
| ADR #5 | Free / Pro / Premium / Enterprise | **Free, Pro, Premium, Enterprise kept** — ADR #5's numeric quotas inform D3. |
| Website + Paddle (live) | Trial / Pro / Premium / Enterprise | **Trial → Free (forever). Plus is new.** Pro/Premium prices unchanged. |
| oz-core enum | Free / OneTime / Standard / Pro / Premium / Enterprise | Legacy `OneTime`/`Standard` drop out (D9). |

---

## 2. Where each definition lives (source map)

| File | Role | Tier source |
| :--- | :--- | :--- |
| `docs/BUSINESS_PLAN.md` §2 | Market/pricing plan (IDR, annual) | 1-Time / Standard / Pro / Enterprise — **superseded for pricing** |
| `docs/decisions/2026-07-10-subscription-tier-entitlement.md` (ADR #5) | Design intent | Free / Pro / Premium / Enterprise with numeric quotas |
| `docs/decisions/2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md` (ADR #23) | Trial lifecycle | 90-day trial — **to be re-scoped to the free-forever tier** (D5) |
| `crates/oz-core/src/subscription.rs` | **Enforcement** (client-side quotas) | enum Free/OneTime/Standard/Pro/Premium/Enterprise |
| `apps/license-server/paddle_webhook.go` → `tierQuotas()` | **Enforcement** (license mint) | `pro/premium/enterprise` → 0/0/all types; `free` → 1/1/3 types |
| `apps/license-server/pb_schema.json` | Schema select values | `free, pro, premium, enterprise` — **needs `plus`** |
| `apps/license-server/renew.go` | Offline renewal expiry | `free` +100y, `pro/premium` +1y, `enterprise` +3y |
| `website/src/content/pricing/{en,id}.ts` | **Live pricing pages** | trial / pro / premium / enterprise (USD $19/$49, Rp display) — **needs `plus` + free-forever card** |
| `apps/license-server` `PADDLE_PRICE_TIERS` (env) | Live billing map | 2 prices today: `pri_…racp:pro`, `pri_…8cec:premium` — **needs 6 once D2 lands** (monthly + yearly × Plus/Pro/Premium) |

Known drift (verified 2026-08-17):
- **Pro quotas:** website says 1 store / 2 registers / 1 warehouse; oz-core
  and `tierQuotas()` enforce **unlimited**. A Pro buyer is currently
  over-entitled vs. what the checkout advertises.
- **`kds` (and `warehouse`) workspace types:** `tierQuotas()` grants all paid
  tiers all 6 types; ADR #5 gates KDS to Premium+.
- **Legacy tiers:** oz-core still carries `OneTime` / `Standard`.
- **Currency:** the business plan is 100% IDR/annual; the live system is USD/
  monthly via Paddle with Rp **display** figures (Paddle cannot bill IDR).

---

## 3. Decisions to finalize

Mark each with ✅ (adopt recommendation), or write your choice in the
**Decision** line. D1 is done; everything else stays open until written here.

### D1 — Tier lineup ✅ **RESOLVED 2026-08-17**

**Free / Plus / Pro / Premium / Enterprise** — five tiers. **Free = free
forever, 1 workspace only.** Pro $19 and Premium $49 keep their Paddle
prices; Plus is a new entry price; Enterprise is bespoke.

- **Decision:** ✅ 5-tier lineup as stated above.

### D2 — Billing currency & frequency ✅ **RESOLVED 2026-08-17**

**Every paid tier is priced in both USD and IDR (display), sold monthly and
yearly.** Yearly prices are rounded to clean figures. **USD yearly is a
uniform ≈17% off** (pay 10 months: $5→$50, $10→$100, $25→$250); **IDR
yearly keeps the increasing 15–25% ladder** (cheaper tiers get less:
≈16% → 21% → 25%).

Worked price points — monthly anchors decided 2026-08-17: **$0 / $5 / $10 /
$25** (Free / Plus / Pro / Premium):

| Tier | USD monthly | USD yearly (≈off) | IDR monthly | IDR yearly (≈off) |
| :--- | :---: | :---: | :---: | :---: |
| **Free** | $0 | — (free forever) | Rp 0 | — |
| **Plus** | **$5** | **$50** (16.7%) | **Rp 79.000** | **Rp 799.000** (15.7%) |
| **Pro** | **$10** | **$100** (16.7%) | **Rp 159.000** | **Rp 1.499.000** (21.4%) |
| **Premium** | **$25** | **$250** (16.7%) | **Rp 399.000** | **Rp 3.599.000** (24.8%) |
| **Enterprise** | Bespoke | Bespoke | Kustom | Kustom |

USD yearly is the classic **pay-10-months** model (10 × monthly, ≈16.7% off)
— flat across tiers by design ($50 / $100 / $250). IDR yearly keeps the
increasing ladder (15.7% → 21.4% → 24.8%). ⚠️ Note the asymmetry: USD gives
every tier the same ≈17%, while IDR gives richer tiers more (up to ≈25%).
If IDR yearly should mirror the flat USD model, it becomes Rp 790.000 /
1.590.000 / 3.990.000 (10 × monthly) instead. These replace the current live
Paddle prices ($19/$49) — the old prices get archived when the new ones go
live.

- **Facts that still hold:** Paddle cannot bill IDR — the checkout always
  charges the USD price (monthly or yearly); the Rp figures are display only.
  A "billed in USD" note goes on the ID pricing page.
- **Implementation:** each (tier × frequency) is its own Paddle price —
  **6 prices total** (Plus/Pro/Premium × monthly/yearly), each mapped in
  `PADDLE_PRICE_TIERS` back to its `tier_key`. Rounding convention used here:
  clean whole dollars (USD, currently pay-10-months = ≈17% off) and classic
  `Rp …9.000` price points (IDR, increasing 15–25% ladder).
- **Decision:** ✅ USD + IDR display × monthly + yearly as tabled; monthly
  anchors **$0 / $5 / $10 / $25**; USD yearly **$50 / $100 / $250**; IDR
  yearly Rp 799.000 / 1.499.000 / 3.599.000 (or the flat 10×-monthly variant
  if USD/IDR should match).

### D3 — Quota & feature matrix (full comparison table)

The core reconciliation. Free is fixed by D1 (**1 workspace only** — one
store, one register, one warehouse). Everything else below is the
**proposal** — adjust any cell. Rows marked † are feature-gate decisions
that **D4** finalizes (value shown = the D4 recommendation). Rows marked *
are not enforced anywhere in the code yet — they must be implemented (or
removed) before shipping.

#### Numeric limits

| Dimension | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Max stores | 1 | 1 | 3 † | Unlimited | Unlimited |
| Max terminals (registers) / store | 1 | 2 | 5 † | Unlimited | Unlimited |
| Max warehouses | 1 | 1 | 3 † | Unlimited | Unlimited |
| Max KDS screens | 0 | 0 | 1 | Unlimited | Unlimited |
| Max staff users * | 1 | 3 | 10 | Unlimited | Unlimited |

#### Workspace types (ADR #4)

| Type | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| `restaurant-pos` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `store-pos` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `admin` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `inventory` | ✗ | ✓ | ✓ | ✓ | ✓ |
| `warehouse` | ✗ | ✓ | ✓ | ✓ | ✓ |
| `kds` | ✗ | ✗ | ✓ | ✓ | ✓ |

#### Payments

| Feature | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Cash & manual split | ✓ | ✓ | ✓ | ✓ | ✓ |
| QRIS (Midtrans) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Stripe cards | ✗ | ✗ | ✓ † | ✓ | ✓ |
| Multi-currency | ✗ | ✗ | ✓ † | ✓ | ✓ |

#### Sync & cloud

| Feature | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Offline-first SQLite engine | ✓ | ✓ | ✓ | ✓ | ✓ |
| Cloud sync (PostgreSQL outbox) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Multi-store dashboard | ✗ | ✗ | ✓ | ✓ | ✓ |
| CSV / data export | ✓ | ✓ | ✓ | ✓ | ✓ |

#### Business logic

| Feature | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Custom tax (PPN / PB1 / service) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Product bundles | ✗ | ✓ | ✓ | ✓ | ✓ |
| Lua scripting | ✗ | ✗ | ✗ | ✓ | ✓ |
| Loyalty tiers & points | ✗ | ✗ | ✗ | ✓ | ✓ |
| Multi-warehouse routing | ✗ | ✗ | ✓ | ✓ | ✓ |
| Live order simulation debugger | ✗ | ✗ | ✓ | ✓ | ✓ |
| AI demand forecasting (roadmap) | ✗ | ✗ | ✗ | ✗ | ✓ |

#### Hardware (HAL)

| Feature | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Scanner / printer / cash drawer | ✓ | ✓ | ✓ | ✓ | ✓ |
| Customer display | ✗ | ✗ | ✓ | ✓ | ✓ |
| KDS hardware | ✗ | ✗ | ✗ | ✓ | ✓ |
| Custom HAL drivers | ✗ | ✗ | ✗ | ✗ | ✓ |

#### Support & platform

| Feature | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Community forum | ✓ | ✓ | ✓ | ✓ | ✓ |
| Email / chat support | ✗ | ✓ | ✓ | ✓ | ✓ |
| Priority support | ✗ | ✗ | ✗ | ✓ | ✓ |
| Software updates | minor only | minor + major | minor + major | minor + major | minor + major |
| White-label branding | ✗ | ✗ | ✗ | ✓ † | ✓ |
| Offline grace period | — (never expires) | 14 days | 14 days | 14 days | custom |
| Enterprise services (dedicated hosting, ERP adaptors, account manager) | ✗ | ✗ | ✗ | ✗ | ✓ |

- **Recommendation:** adopt the matrix above (adjust freely) and update the
  enforcement to match: `tierQuotas()` and `SubscriptionTier::max_*()` must
  encode the same numbers as the pricing pages, or the drift resurfaces.
- **Decision:** __________

### D4 — Feature gates (what unlocks where)

- **Live site:** trial = nothing; **Pro** = QRIS + cloud sync; **Premium** =
  + Lua + priority support. Stripe cards are implemented in `oz-payment` but
  not listed on the site.
- **Open sub-questions:**
  1. **Plus features** — proposed: QRIS + cloud sync (the old Pro feature
     set, at a lower price). Confirm.
  2. **Stripe card payments** — Pro or Premium+? (recommend **Pro+**, giving
     Pro a capability edge over Plus beyond quantity).
  3. **KDS** — ✅ decided with D3: **Pro gets up to 1 KDS screen**, Premium+
     unlimited. Enforcement (currently all paid tiers get `kds` with no cap)
     must limit Pro to 1.
  4. **Lua scripting** — Pro (business plan) vs Premium+ (site)? (recommend
     **Premium+**, as shipped).
  5. **Loyalty/points** (business plan Pro+) — which tier? (recommend
     **Premium+**, matches "automation" positioning).
- **Decision:** __________

### D5 — Free tier lifecycle (was: trial terms)

- **Decided:** Free is **free forever**, not a 90-day trial — no trial clock,
  no countdown banners.
- **Still to confirm:**
  1. Keep the **hardware-fingerprint anti-abuse lock** (ADR #23) on the free
     tier so reinstalls can't spawn unlimited free workspaces? (recommend
     **yes** — same mechanism, no expiry).
  2. Retire the 90-day / 14-day-grace lifecycle from ADR #23, or keep it as a
     future promotional offer? (recommend **retire** — Free is now the
     permanent entry tier).
- **Decision:** __________

### D6 — Renewal & grace semantics

- **Live:** Paddle renews monthly; key/subscription expiry mirrors the Paddle
  billing period (`subscriptionTimes`). Offline grace is 14 days per ADR #5/#23.
- **Open question:** `renew.go` (manual offline re-activation path) extends
  `pro/premium` by **1 year** and `enterprise` by **3 years** — inconsistent
  with monthly billing. Pick one:
  - **(a)** Offline re-activation extends by the **same billing period** as the
    Paddle subscription (monthly) — most consistent.
  - **(b)** Keep +1y/+3y as a deliberate "goodwill" policy.
- **Recommendation:** **(a)**.
- **Decision:** __________

### D7 — One-time perpetual licensing

- **Status:** the business plan's 1-Time tier is retired by D1; a perpetual
  option is no longer part of the lineup. **Closed** unless a paying segment
  asks for it.
- **Decision:** ✅ Closed — no perpetual tier.

### D8 — Enterprise definition

- **Business plan:** bespoke annual contract, dedicated/private hosting, custom
  ERP adaptors (SAP/Odoo), account manager, on-site training.
- **Live:** custom (mailto `sales@oz-pos.com`), no Paddle price, unlimited
  quotas.
- **Recommendation:** keep the mailto/contact path as the only sales channel
  for now; define the enterprise offering contractually per-customer. No code
  change needed beyond confirming quotas = unlimited.
- **Decision:** __________

### D9 — Tier-key cleanup (schema & enum)

- **Target values:** `free, plus, pro, premium, enterprise` in
  `pb_schema.json` select fields, `SubscriptionTier::from_db()`, and the
  pricing `TierKey` union. Drop legacy `trial`, `standard`, `one_time`,
  `perpetual` aliases (keep accepting them defensively in `from_db` is fine,
  but stop documenting them as tiers).
- **Decision:** __________

---

## 4. Proposed final matrix (once D2–D9 are decided)

This is the target single source of truth. It is **proposed**, not final —
fill in §3 first.

| | Free (forever) | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Price (USD/mo)** | $0 | $5 | $10 | $25 | Custom |
| **Price (USD/yr)** | — | $50 | $100 | $250 | — |
| **Price (IDR/mo)** | Rp 0 | Rp 79.000 | Rp 159.000 | Rp 399.000 | Kustom |
| **Price (IDR/yr)** | — | Rp 799.000 | Rp 1.499.000 | Rp 3.599.000 | — |
| **Billing** | — | Paddle, monthly or yearly | Paddle, monthly or yearly | Paddle, monthly or yearly | Bespoke contract |
| **License key** | — (free, hw-locked) | `OZ-PLUS-…` | `OZ-PRO-…` | `OZ-PREMIUM-…` | `OZ-ENTERPRISE-…` |

> Prices are finalized in **D2 (§3)**. The full quota & feature comparison
> matrix — numeric limits, workspace types, payments, sync, business logic,
> hardware, support (≈35 rows) — lives in **§3 D3** and is the proposal
> pending confirmation.

---

## 5. Implementation ripple (what changes once decided)

| # | Change | File(s) |
| :--- | :--- | :--- |
| 1 | Add `plus` to the tier enum + quota methods | `crates/oz-core/src/subscription.rs` |
| 2 | Add `plus` case to `tierQuotas()` | `apps/license-server/paddle_webhook.go` |
| 3 | Add `plus` to schema select values | `apps/license-server/pb_schema.json` (license_keys, subscriptions, tenants?) |
| 4 | Rework pricing pages: Free card = "free forever / 1 workspace", new Plus card | `website/src/content/pricing/{en,id}.ts` + `types.ts` (TierKey union) |
| 5 | Create the **6 Paddle prices** (Plus/Pro/Premium × monthly/yearly) at the D2 points + wire them; archive the old $19/$49 | Paddle dashboard → `PADDLE_PRICE_TIERS` env (6 mappings) → `docs/operations/go-live-checklist.md` |
| 11 | Pricing pages: monthly/yearly toggle, per-frequency priceIds, "billed in USD" note (ID) | `website/src/content/pricing/{en,id}.ts` + `CheckoutButton.tsx` (pass the chosen frequency's priceId) |
| 6 | Align enforcement with the decided quotas (fix Pro over-entitlement) | `tierQuotas()` + `SubscriptionTier::max_*()` |
| 7 | Free-tier lifecycle (hw lock, no trial clock) | ADR #23 re-scope; client trial timer code |
| 8 | Renewal expiry policy (D6) | `apps/license-server/renew.go` |
| 9 | Supersede stale docs | `docs/BUSINESS_PLAN.md` §2, ADR #5 quota table, `website-plan.md` tier section |
| 10 | Regression tests for the new quota table | `apps/license-server/paddle_webhook_test.go`, `handler_test.go`, oz-core subscription tests |

---

## 6. Evidence & decision log

**2026-08-17 — D1 resolved:** five-tier lineup `Free / Plus / Pro / Premium /
Enterprise`; Free = free forever, 1 workspace only; Plus is a new entry tier;
Pro/Premium keep their live Paddle prices; Enterprise stays bespoke.

**2026-08-17 — D3 partial:** the numeric-limit and workspace-type matrix was
expanded to a full ~35-row comparison table; Pro gets **max 1 KDS screen**
(so the `kds` workspace type unlocks on Pro, unlimited on Premium+). Remaining
numeric cells (stores/terminals/warehouses, staff users) still open.

**2026-08-17 — sandbox purchase verified end-to-end:** Paddle checkout → test
payment (`4242 4242 4242 4242`) → webhook events → transaction
`txn_01m07za3pygc0qdj464hdq66ev` **completed** → subscription
`sub_01m07zaeebjxff7cfawmpmeqxn` **active** on Pro `pri_01m05gdnqp30xze6db73qcracp`.
The only two Paddle prices in existence are Pro $19 and Premium $49 (sandbox).

**Urgent drift to resolve first (D3):** `tierQuotas()` currently grants
**unlimited** quotas to all paid tiers — the marketing pages promise less, and
the new Plus tier makes the gap bigger. Align enforcement with the decided
table before the Plus price goes live.
