# Subscription Tier Finalization

> **Status:** DRAFT — this document exists to force the remaining tier decisions.
> **Date:** 2026-08-17
> **Why this exists:** the repo carries several overlapping (and mutually
> contradictory) tier definitions — the business plan, ADR #5, the oz-core
> entitlement enum, and the live website + Paddle implementation. Before we
> harden anything else, the tier list itself must be one source of truth.
> Work through **§3 (the decisions)** — every open question has the options
> already discussed somewhere in the repo, with a recommendation.

---

## 1. The conflict in one table

The same tier name means different things in different files today:

| Tier | BUSINESS_PLAN.md (market) | ADR #5 (design, 2026-07) | oz-core `SubscriptionTier` (enforcement) | Website + Paddle (live, shipping) |
| :--- | :--- | :--- | :--- | :--- |
| **Free / Trial** | 3-month offline-only trial, 1 store / 1 terminal / 1 wh | 1 store, 1 register, `store-pos`+`admin` | 1 store, 1 instance, 1 wh, `[restaurant-pos, store-pos, admin]` | 90-day trial card, no checkout, no QRIS/cloud/Lua |
| **1-Time** | IDR 3.5jt one-time / terminal, perpetual | — (legacy enum) | 1 store, 1 instance | not offered ("roadmap") |
| **Standard** | IDR 2jt / yr, 1 store / 2 terminals / 1 wh | — (legacy enum) | 1 store, **2** instances | not offered |
| **Pro** | IDR 5jt / yr, **unlimited** everything, QRIS + Stripe + Lua | **2 stores, 3 registers/store**, + `inventory` | **unlimited** stores/instances/wh, all types | **$19/mo**, 1 store, 2 registers, 1 wh, QRIS + cloud, **no Lua** |
| **Premium** | — (not a business-plan tier) | **5 stores, 10 registers/store**, + `kds`, `analytics-pro` | unlimited, all types | **$49/mo**, unlimited everything + Lua |
| **Enterprise** | Bespoke, dedicated infra, ERP adaptors | Unlimited, all + custom plugins | unlimited, all types | Custom (mailto), no Paddle price |

Four sources, four different answers for "what does Pro get".

---

## 2. Where each definition lives (source map)

| File | Role | Tier source |
| :--- | :--- | :--- |
| `docs/BUSINESS_PLAN.md` §2 | Market/pricing plan (IDR, annual) | 1-Time / Standard / Pro / Enterprise |
| `docs/decisions/2026-07-10-subscription-tier-entitlement.md` (ADR #5) | Design intent | Free / Pro / Premium / Enterprise with numeric quotas |
| `docs/decisions/2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md` (ADR #23) | Trial lifecycle | 90-day trial, 14-day grace, hardware fingerprint lock |
| `crates/oz-core/src/subscription.rs` | **Enforcement** (client-side quotas) | enum Free/OneTime/Standard/Pro/Premium/Enterprise |
| `apps/license-server/paddle_webhook.go` → `tierQuotas()` | **Enforcement** (license mint) | `pro/premium/enterprise` → 0/0/all types; `free` → 1/1/3 types |
| `apps/license-server/pb_schema.json` | Schema select values | `free, pro, premium, enterprise` |
| `apps/license-server/renew.go` | Offline renewal expiry | `free` +100y, `pro/premium` +1y, `enterprise` +3y |
| `website/src/content/pricing/{en,id}.ts` | **Live pricing pages** | trial / pro / premium / enterprise (USD $19/$49, Rp display) |
| `apps/license-server` `PADDLE_PRICE_TIERS` (env) | Live billing map | 2 prices: `pri_…racp:pro`, `pri_…8cec:premium` |

Known drift (verified 2026-08-17):
- **Pro quotas:** website says 1 store / 2 registers / 1 warehouse; oz-core
  and `tierQuotas()` enforce **unlimited**. A Pro buyer is currently
  over-entitled vs. what the checkout advertises.
- **`kds` (and `warehouse`) workspace types:** `tierQuotas()` grants all paid
  tiers all 6 types; ADR #5 gates KDS to Premium+.
- **Legacy tiers:** oz-core still carries `OneTime` / `Standard` (business-plan
  era) that no longer exist in the schema select or the site.
- **Currency:** the business plan is 100% IDR/annual; the live system is USD/
  monthly via Paddle with Rp **display** figures (Paddle cannot bill IDR).

---

## 3. Decisions to finalize

Mark each with ✅ (adopt recommendation), or write your choice in the
**Decision** line. Until every row has a decision, this document stays DRAFT.

### D1 — Tier lineup

- **Options:**
  - **(a) Keep the live lineup:** Free trial / Pro / Premium / Enterprise.
  - **(b) Business-plan lineup:** 1-Time / Standard / Pro / Enterprise.
  - **(c) Hybrid:** live lineup **plus** an optional one-time perpetual tier.
- **Shipped today:** (a) — Pro $19 and Premium $49 are the only Paddle prices;
  1-Time/Standard were never wired to billing.
- **Recommendation:** **(a)**. Standard/1-Time add pricing complexity and a
  perpetual tier conflicts with the subscription-first roadmap. Keep the
  one-time perpetual idea in the roadmap (see D7).
- **Decision:** __________

### D2 — Billing currency & frequency

- **Facts:** Paddle supports no IDR; the site currently charges the USD price
  and shows an Rp figure (Rp 299.000 ≈ $19, Rp 749.000 ≈ $49). Business plan
  priced everything in IDR **annual**.
- **Options:**
  - **(a) USD monthly only** (live today) — Paddle handles tax/MoR, receipt
    emails, refunds. Rp stays display-only; add a "billed in USD" note on the
    ID page.
  - **(b) USD monthly + annual** (annual = 2 months free, e.g. $190/yr) —
    needs 2 more Paddle prices and 2 more `PADDLE_PRICE_TIERS` mappings.
  - **(c) True IDR billing** via a local provider (Midtrans/Xendit) alongside
    Paddle — significant work; needed only if IDR card/QRIS billing is a hard
    requirement.
- **Recommendation:** **(a)** now, **(b)** as the first pricing expansion, **(c)**
  only when a paying IDR segment demands it.
- **Decision:** __________

### D3 — Quota table (stores / registers / warehouses / workspace types)

The core reconciliation. Options are the three published matrices:

| | BUSINESS_PLAN (Pro) | ADR #5 (Pro / Premium) | Website live (Pro / Premium) |
| :--- | :--- | :--- | :--- |
| Stores | Unlimited | 2 / 5 | 1 / Unlimited |
| Registers | Unlimited | 3 / 10 | 2 / Unlimited |
| Warehouses | Unlimited | — / — | 1 / Unlimited |
| Workspace types | — | +`inventory` / +`kds` | all paid = all 6 types (current code) |

- **Recommendation:** adopt the **website matrix as written** (it is what the
  buyer sees at checkout — marketing and enforcement must match) and update
  the enforcement to match:
  - **Free/trial:** 1 store, 1 register, 1 warehouse — `restaurant-pos`,
    `store-pos`, `admin`.
  - **Pro:** 1 store, 2 registers, 1 warehouse — + `inventory`, `warehouse`.
  - **Premium:** unlimited stores/registers/warehouses — + `kds`.
  - **Enterprise:** unlimited — all types.
  - Concretely: `tierQuotas()` and `SubscriptionTier::max_*()` must encode the
    same numbers, or the drift resurfaces.
- **Decision:** __________

### D4 — Feature gates (what unlocks where)

- **Live site:** trial = nothing; **Pro** = QRIS + cloud sync; **Premium** =
  + Lua + priority support. Stripe cards are implemented in `oz-payment` but
  not listed on the site.
- **Business plan:** QRIS on Standard+, Stripe on Pro+, Lua on Pro+.
- **Open sub-questions:**
  1. **Stripe card payments** — Pro or Premium+? (recommend **Premium+**, so
     Pro ≠ Premium only by quantity; differentiate on capability).
  2. **KDS** — Premium+ (ADR #5) vs all paid (current code)? (recommend
     **Premium+**).
  3. **Lua scripting** — Pro (business plan) vs Premium+ (site)? (recommend
     **Premium+**, as shipped).
  4. **Loyalty/points** (business plan Pro+) — in scope for which tier?
     (recommend **Premium+**, matches "automation" positioning).
- **Decision:** __________

### D5 — Trial terms

- **Settled in ADR #23** (adopt as final): **90-day trial**, 1 store/1
  register/1 warehouse, offline-only (no cloud sync, no payment gateways),
  hardware-fingerprint anti-abuse lock, expiry warning at day 76, 14-day
  offline grace, then soft lock with upgrade path.
- **Confirm:** trial has **no Paddle price** and the site's Free card must
  keep no `priceId` (checkout must stay disabled for it).
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

- **Options:** ship it (business-plan 1-Time tier), or keep deferred.
- **Shipped today:** deferred — site copy literally says "One-time (perpetual)
  licensing is not available yet — see the roadmap."
- **Recommendation:** keep **deferred**; re-open only after subscription
  churn/retention data justifies it.
- **Decision:** __________

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

- **Current:** schema select = `free, pro, premium, enterprise`; oz-core enum
  still maps legacy `standard`, `one_time`, `perpetual` strings.
- **Recommendation:** after D1, drop the legacy aliases from
  `SubscriptionTier::from_db` (keep accepting them defensively is fine, but
  stop documenting them as tiers).
- **Decision:** __________

---

## 4. Proposed final matrix (once D1–D9 are decided)

This is the target single source of truth. It is **proposed**, not final —
fill in §3 first.

| | Free (90-day trial) | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: |
| **Price (USD/mo)** | $0 | $19 | $49 | Custom |
| **Price (IDR display)** | Rp 0 | Rp 299.000 | Rp 749.000 | Kustom |
| **Billing** | — | Paddle, monthly, auto-renew | Paddle, monthly, auto-renew | Bespoke contract |
| **Stores** | 1 | 1 | Unlimited | Unlimited |
| **Registers** | 1 | 2 | Unlimited | Unlimited |
| **Warehouses** | 1 | 1 | Unlimited | Unlimited |
| **Workspace types** | `restaurant-pos`, `store-pos`, `admin` | + `inventory`, `warehouse` | + `kds` | all |
| **QRIS (Midtrans)** | ✗ | ✓ | ✓ | ✓ |
| **Stripe cards** | ✗ | ? (D4.1) | ✓ | ✓ |
| **Cloud sync** | ✗ | ✓ | ✓ | ✓ |
| **Lua scripting** | ✗ | ✗ | ✓ | ✓ |
| **Priority support** | ✗ | ✗ | ✓ | ✓ (+ AM) |
| **License key** | — (trial, hw-locked) | `OZ-PRO-…` | `OZ-PREMIUM-…` | `OZ-ENTERPRISE-…` |

---

## 5. Implementation ripple (what changes once decided)

| # | Change | File(s) |
| :--- | :--- | :--- |
| 1 | Quota numbers per tier | `apps/license-server/paddle_webhook.go` → `tierQuotas()`; `crates/oz-core/src/subscription.rs` → `max_stores()/max_pos_instances()/max_warehouses()/allows_workspace_type()` |
| 2 | Feature rows (Stripe, KDS, loyalty) | `website/src/content/pricing/{en,id}.ts` + `types.ts` |
| 3 | New prices (annual, if D2=b) | Paddle dashboard → 2 new prices → `PADDLE_PRICE_TIERS` env → docs/operations/go-live-checklist.md |
| 4 | IDR display note ("billed in USD") | `website/src/content/pricing/id.ts` |
| 5 | Tier-key cleanup (D9) | `crates/oz-core/src/subscription.rs` `from_db()`; verify `pb_schema.json` select values |
| 6 | Renewal expiry policy (D6) | `apps/license-server/renew.go` |
| 7 | Supersede stale docs | `docs/BUSINESS_PLAN.md` §2 (mark superseded for pricing); ADR #5 quota table (amend); `website-plan.md` tier section |
| 8 | Regression tests | `apps/license-server/paddle_webhook_test.go`, `handler_test.go` (quota assertions), oz-core subscription tests |

---

## 6. Evidence recorded 2026-08-17

- The full sandbox purchase loop is verified live: Paddle checkout → test
  payment (`4242 4242 4242 4242`) → webhook events → transaction
  `txn_01m07za3pygc0qdj464hdq66ev` **completed** → subscription
  `sub_01m07zaeebjxff7cfawmpmeqxn` **active** on Pro `pri_01m05gdnqp30xze6db73qcracp`.
- The only two Paddle prices in existence are Pro $19 and Premium $49 (sandbox).
- `tierQuotas()` currently grants **unlimited** quotas to all paid tiers —
  the marketing pages promise less. This is the single most urgent drift to
  resolve once D3 is decided.
