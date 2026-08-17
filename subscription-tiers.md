# Subscription Tiers — Final Decisions

> Approved 2026-08-17. Single source of truth for tier pricing, quotas, and
> feature gates. Supersedes the tier/pricing sections of `docs/BUSINESS_PLAN.md`
> §2, ADR #5, and the older pricing content until those are updated to match.

## 1. Lineup

**Five tiers: Free · Plus · Pro · Premium · Enterprise**

| Tier | Position |
| :--- | :--- |
| **Free** | Free forever — 1 workspace only (1 store, 1 terminal, 1 warehouse) |
| **Plus** | Entry paid tier |
| **Pro** | Mid paid tier |
| **Premium** | Top paid tier |
| **Enterprise** | Bespoke — no list price, contact sales |

## 2. Pricing

USD and IDR are **independent market prices**: global customers pay the USD
rate; Indonesian customers pay the IDR rate (lower, set for the local
market). Paddle bills in USD only — the IDR rates are honored via Paddle
country price overrides for Indonesia (the customer's charge ≈ the Rp figure).

| Tier | USD/mo | USD/yr (≈off) | IDR/mo | IDR/yr (≈off) |
| :--- | :---: | :---: | :---: | :---: |
| **Free** | $0 | — | Rp 0 | — |
| **Plus** | $5 | $50 (16.7%) | Rp 49.000 | Rp 500.000 (15.0%) |
| **Pro** | $10 | $100 (16.7%) | Rp 99.000 | Rp 1.000.000 (15.8%) |
| **Premium** | $25 | $250 (16.7%) | Rp 199.000 | Rp 2.000.000 (16.2%) |
| **Enterprise** | Bespoke | Bespoke | Kustom | Kustom |

Yearly = pay 10 months (10 × monthly, ≈15–17% off) in both currencies.
Six Paddle prices total (Plus/Pro/Premium × monthly/yearly); the IDR rates
map to country-specific overrides for Indonesia.

## 3. Quota & feature matrix

TBD = not yet decided (Pro store/terminal/warehouse counts).

### Numeric limits

| Dimension | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Max stores | 1 | 1 | TBD | Unlimited | Unlimited |
| Max terminals (registers) / store | 1 | 2 | TBD | Unlimited | Unlimited |
| Max warehouses | 1 | 1 | TBD | Unlimited | Unlimited |
| Max KDS screens | 0 | 0 | 1 | Unlimited | Unlimited |

### Workspace types

| Type | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| `restaurant-pos` / `store-pos` / `admin` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `inventory` / `warehouse` | ✗ | ✓ | ✓ | ✓ | ✓ |
| `kds` | ✗ | ✗ | ✓ | ✓ | ✓ |

### Payments

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Cash & manual split | ✓ | ✓ | ✓ | ✓ | ✓ |
| QRIS (Midtrans) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Stripe cards | ✗ | ✗ | ✓ | ✓ | ✓ |
| Multi-currency | ✗ | ✗ | ✓ | ✓ | ✓ |

### Sync & cloud

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Offline-first SQLite engine | ✓ | ✓ | ✓ | ✓ | ✓ |
| Cloud sync (PostgreSQL outbox) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Multi-store dashboard | ✗ | ✗ | ✓ | ✓ | ✓ |
| CSV / data export | ✓ | ✓ | ✓ | ✓ | ✓ |

### Business logic

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Custom tax (PPN / PB1 / service) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Product bundles | ✗ | ✓ | ✓ | ✓ | ✓ |
| Lua scripting | ✗ | ✗ | ✗ | ✓ | ✓ |
| Loyalty tiers & points | ✗ | ✗ | ✗ | ✓ | ✓ |
| Multi-warehouse routing | ✗ | ✗ | ✓ | ✓ | ✓ |
| Live order simulation debugger | ✗ | ✗ | ✓ | ✓ | ✓ |
| AI demand forecasting (roadmap) | ✗ | ✗ | ✗ | ✗ | ✓ |

### Hardware (HAL)

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Scanner / printer / cash drawer | ✓ | ✓ | ✓ | ✓ | ✓ |
| Customer display | ✗ | ✗ | ✓ | ✓ | ✓ |
| KDS hardware | ✗ | ✗ | ✓ | ✓ | ✓ |
| Custom HAL drivers | ✗ | ✗ | ✗ | ✗ | ✓ |

### Support & platform

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Community forum | ✓ | ✓ | ✓ | ✓ | ✓ |
| Email / chat support | ✗ | ✓ | ✓ | ✓ | ✓ |
| Priority support | ✗ | ✗ | ✗ | ✓ | ✓ |
| Software updates | minor only | minor + major | minor + major | minor + major | minor + major |
| White-label branding | ✗ | ✗ | ✗ | ✓ | ✓ |
| Offline grace period | — (never expires) | 14 days | 14 days | 14 days | custom |
| Enterprise services (dedicated hosting, ERP adaptors, account manager) | ✗ | ✗ | ✗ | ✗ | ✓ |

---

## 4. Where each definition lives (source map)

| File | Role | Tier source |
| :--- | :--- | :--- |
| `docs/BUSINESS_PLAN.md` §2 | Market/pricing plan (IDR, annual) | 1-Time / Standard / Pro / Enterprise — superseded for pricing |
| `docs/decisions/2026-07-10-subscription-tier-entitlement.md` (ADR #5) | Design intent | Free / Pro / Premium / Enterprise with numeric quotas |
| `docs/decisions/2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md` (ADR #23) | Trial lifecycle | 90-day trial — to be re-scoped to the free-forever tier |
| `crates/oz-core/src/subscription.rs` | Enforcement (client-side quotas) | enum Free/OneTime/Standard/Pro/Premium/Enterprise |
| `apps/license-server/paddle_webhook.go` → `tierQuotas()` | Enforcement (license mint) | pro/premium/enterprise → 0/0/all types; free → 1/1/3 types |
| `apps/license-server/pb_schema.json` | Schema select values | free, pro, premium, enterprise — needs plus |
| `apps/license-server/renew.go` | Offline renewal expiry | free +100y, pro/premium +1y, enterprise +3y |
| `website/src/content/pricing/{en,id}.ts` | Live pricing pages | trial / pro / premium / enterprise (USD $19/$49, Rp display) — needs plus + free-forever card |
| `apps/license-server` `PADDLE_PRICE_TIERS` (env) | Live billing map | 2 prices today: `pri_…racp:pro`, `pri_…8cec:premium` — needs 6 once D2 lands (monthly + yearly × Plus/Pro/Premium) |
