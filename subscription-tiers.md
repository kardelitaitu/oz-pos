# Subscription Tiers — Final Decisions

> **Status: FINAL** — Approved 2026-08-17. Single source of truth for tier
> pricing, quotas, and feature gates. Supersedes the tier/pricing sections of
> `docs/BUSINESS_PLAN.md` §2, ADR #5, and the older pricing content until
> those are updated to match.

## 1. Lineup

**Five tiers: Free · Plus · Pro · Premium · Enterprise**

| Tier | Position |
| :--- | :--- |
| **Free** | Free forever — 1 workspace only (1 store, 1 terminal, 1 warehouse, 30-day sales history) |
| **Plus** | Entry paid tier — hero feature: **Daily Sales Dashboard** (Laporan Harian) |
| **Pro** ⭐ **Most Popular** | Mid paid tier — best for growing single-to-multi-store businesses |
| **Premium** | Top paid tier — multi-store chains with loyalty & automation |
| **Enterprise** | Bespoke — no list price, contact sales |

## 2. Pricing

USD and IDR are **independent market prices**: global customers pay the USD
rate; Indonesian customers pay the IDR rate (lower, set for the local
market). See **Payment routing** below for how each is charged.

| Tier | USD/mo | USD/yr (**2 months free**) | IDR/mo | IDR/yr (**2 bulan gratis**) |
| :--- | :---: | :---: | :---: | :---: |
| **Free** | $0 | — | Rp 0 | — |
| **Plus** | $4.99 | $49.99 | Rp 49.000 | Rp 500.000 |
| **Pro** ⭐ | $9.99 | $99.99 | Rp 99.000 | Rp 1.000.000 |
| **Premium** | $19.99 | $199.99 | Rp 199.000 | Rp 2.000.000 |
| **Enterprise** | Bespoke | Bespoke | Kustom | Kustom |

Yearly = **2 months free** (pay 10 months, get 12). Always market as "2 months free" /
"2 bulan gratis" — not as a percentage discount. Annual plan must be the **default
selection** on the pricing page; users actively switch to monthly.
Six Paddle prices total (Plus/Pro/Premium × monthly/yearly).

### Enterprise Pricing Guidance

Enterprise pricing should be defined within these ranges to ensure consistency:
- **Small Enterprise (5-20 stores):** $100-200/mo or Rp 1.000.000-2.000.000/mo
- **Medium Enterprise (21-100 stores):** $200-400/mo or Rp 2.000.000-4.000.000/mo
- **Large Enterprise (100+ stores):** $400+/mo or Rp 4.000.000+/mo

Final pricing determined by: number of stores, terminals, users, support level, and custom integrations required.

### Payment routing

| Market | Provider | Currency | Payment methods |
| :--- | :--- | :--- | :--- |
| **Global** | Paddle (MoR) | USD | cards |
| **Indonesia** | **Midtrans** (Phase 2) | IDR, fixed Rp | QRIS, virtual accounts, e-wallets, cards |

- **Phase 1 (now):** Paddle for everyone. The IDR rates are honored via
  Paddle country price overrides for Indonesia — Paddle geolocates the
  buyer's IP at checkout and applies the override for the country selected.
  IDR isn't a supported currency, so the override is a USD amount ≈ the Rp
  figure (e.g. Premium yearly ≈ $125), which drifts with FX.
- **Phase 2 (next) — CRITICAL revenue unlock:** route Indonesian customers to a
  **Midtrans** checkout — fixed Rp prices and local payment methods (QRIS, virtual
  accounts, e-wallets) that cards alone can't reach; Paddle stays for global.
  Midtrans over Xendit because `oz-payment` already integrates Midtrans QRIS
  for in-store payments. **Without Phase 2, the Indonesian TAM is effectively limited
  to card-holding customers — a fraction of the 65M MSME market. This is not optional
  for Indonesian revenue growth.**
- **Costs of Phase 2:** OZ-POS becomes merchant of record for ID payments
  (Indonesian PPN, refunds, disputes); a second webhook + provisioning path
  in the license server; local-method subscriptions are less mature than
  card auto-renew.

## 3. Quota & feature matrix

### Quick Reference: Best For

> **Positioning statement:** OZ-POS is the QRIS-native POS with offline-first reliability,
> priced for the Indonesian market. Lead every ad, landing page, and sales conversation
> with this — not tier names.

| Tier | Best For | Hero Feature |
| :--- | :--- | :--- |
| **Free** | Warung / kios trying OZ-POS — limited to 30 days of sales history | Cash POS + receipt printing |
| **Plus** | Single-store shops ready to grow from manual to smart | **Daily Sales Dashboard** (Laporan Harian) + QRIS |
| **Pro** ⭐ | Cafes, toko, growing businesses ready for full analytics & KDS | Analytics + KDS + multi-terminal |
| **Premium** | Multi-store chains needing loyalty & automation | Loyalty program + unlimited stores + 1h support |
| **Enterprise** | Large organizations needing white-label, custom hardware & dedicated support | Account manager + custom HAL drivers |

### Numeric Limits

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Max stores | 1 | 1 | 2 | Unlimited | Unlimited |
| Max terminals (registers) / store | 1 | 2 | 5 | Unlimited | Unlimited |
| Max warehouses | 1 | 2 | 3 | Unlimited | Unlimited |
| Max KDS screens | 0 | 0 | 1 / store | Unlimited | Unlimited |
| Max staff users * | 1 | 5 | 20 | Unlimited | Unlimited |
| Sales history (view & export) ** | 30 days | Unlimited | Unlimited | Unlimited | Unlimited |

\* Max staff users — **MUST be enforced before launch** to prevent revenue leakage.

\*\* Sales history cap — **MUST be enforced before launch**. Free users see only the last
30 days of transactions. After 30+ days of use, the owner naturally wants to compare
months — that is the primary upgrade trigger for Free → Plus. Show a blurred/locked
history preview with an upgrade CTA, not a hard error.

### Workspace Types

| Feature | Free | Plus | Pro | Premium | Enterprise |
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

### Sync & Cloud

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Offline-first SQLite engine | ✓ | ✓ | ✓ | ✓ | ✓ |
| Cloud sync (PostgreSQL outbox) | ✗ | ✓ | ✓ | ✓ | ✓ |
| Multi-store dashboard | ✗ | ✗ | ✓ | ✓ | ✓ |
| CSV / data export | ✓ | ✓ | ✓ | ✓ | ✓ |

### Business Logic

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Custom tax (PPN / PB1 / service) | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Daily Sales Dashboard** (Laporan Harian) — Plus hero; show blurred teaser to Free | ✗ | ✓ | ✓ | ✓ | ✓ |
| Reports & analytics (`analytics:view`) | ✗ | ✗ | ✓ | ✓ | ✓ |
| Scheduled report emails | ✗ | ✗ | ✗ | ✓ | ✓ |
| Product bundles | ✗ | ✓ | ✓ | ✓ | ✓ |
| Lua scripting | ✗ | ✗ | ✗ | ✓ | ✓ |
| **Loyalty tiers & points** — show locked animated teaser to Pro users | ✗ | ✗ | ✗ | ✓ | ✓ |
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

### Support & Platform

| Feature | Free | Plus | Pro | Premium | Enterprise |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Community forum | ✓ | ✓ | ✓ | ✓ | ✓ |
| Email / chat support | ✗ | ✓ | ✓ | ✓ | ✓ |
| Priority support | ✗ | ✗ | ✗ | ✓ | ✓ |
| Support response SLA | — | 24h | 8h | 1h (24/7) | account manager |
| Software updates | minor + major | minor + major | minor + major | minor + major | minor + major |
| White-label branding | ✗ | ✗ | ✗ | ✗ | ✓ |
| Offline grace period | 7 days | 14 days | 14 days | 30 days | custom |
| Enterprise services (dedicated hosting, ERP adaptors, account manager) | ✗ | ✗ | ✗ | ✗ | ✓ |

---

## 4. Trial & Conversion Strategy

### Free Tier Trial Flow
- Free tier provides permanent access to basic features (1 store, 1 terminal, 1 staff, **30-day sales history**)
- Trial offer is **segmented by signup vertical** — do NOT offer Pro trial universally:
  a Pro trial anchors users to features they won't pay for and suppresses Plus conversion.

| Signup segment | Trial offer | Rationale |
| :--- | :---: | :--- |
| General signup (no vertical detected) | **14-day Plus trial** | Exposes QRIS + Daily Sales Dashboard — the two hooks that drive Plus conversion |
| Restaurant / cafe landing page | **14-day Pro trial** | KDS is the key differentiator; Pro is the natural landing tier for this vertical |
| Referred by an Enterprise customer | **30-day Pro trial** | High-intent lead; longer trial cost is justified |

- After trial ends: show a clear downgrade screen listing exactly what the user loses,
  with a one-click upgrade path — not a generic error page.

### Trial-to-Paid Conversion — General / Plus Trial (14-day)
- **Day 3:** In-app highlight of Daily Sales Dashboard — show yesterday's summary prominently
- **Day 7:** Email — *"Here's what your first week looked like"* (personalized sales totals)
- **Day 10:** In-app QRIS stats — how many digital payments were processed during the trial
- **Day 14 (last day):** Final email — 30-day history limit warning + upgrade CTA
- **Day 14 (end of trial):** Downgrade screen with blurred history preview and
  *"Upgrade ke Plus untuk tetap melihat riwayat penjualan Anda"*

### Trial-to-Paid Conversion — Pro Trial / Restaurant & Cafe Vertical (14-day)
- **Day 3:** In-app KDS setup guide + first order simulation
- **Day 7:** Email — *"X orders went through your kitchen display this week"*
- **Day 14:** In-app analytics showing peak hours chart + Pro upgrade CTA
- **Day 14 (end of trial):** Downgrade screen — *"KDS akan dinonaktifkan. Upgrade ke Pro untuk melanjutkan."*

### Upgrade/Downgrade Policy
- **Upgrades:** Effective immediately; prorated billing for remainder of current cycle
- **Downgrades:** Effective at end of current billing cycle; no partial refunds
- **Downgrade grace period:** 14 days to export data before feature restrictions apply

---

## 5. Vertical Go-to-Market Strategy

> Customers do not self-identify as "Pro tier users." They identify as cafe owners,
> minimarket operators, or salon managers. All GTM — landing pages, ads, onboarding
> flows — must lead with vertical language, not tier names.

### Vertical × Tier Map

| Vertical | Entry Hook | Natural Tier | Key Features to Highlight |
| :--- | :--- | :---: | :--- |
| **Warung / Kios** | Free → QRIS + daily summary | Plus | Daily Sales Dashboard, QRIS, cloud backup |
| **Kafe / Coffee shop** | KDS demo | Pro | KDS, analytics, multi-terminal |
| **Toko / Minimarket** | Inventory + multi-terminal | Pro | Multi-terminal, warehouse, stock visibility |
| **Salon / Laundry** | Staff & receipt management | Plus/Pro | Staff management, product bundles |
| **Restoran / Rumah Makan** | KDS + loyalty | Pro → Premium | KDS, loyalty points, scheduled reports |
| **Retail chain** | Multi-store ops | Premium | Unlimited stores, analytics, priority support |

### Vertical Landing Pages (Month 1-3 priority)
Create dedicated landing pages per vertical — higher-converting than a generic pricing page:
- `/untuk-kafe` — leads with KDS demo video → Pro trial CTA
- `/untuk-minimarket` — leads with inventory + multi-terminal → Pro trial CTA
- `/untuk-warung` — leads with QRIS + daily summary → Plus CTA
- `/untuk-restoran` — leads with loyalty + automation → Premium CTA

### Vertical-Specific Bundles (Month 3-6)
| Bundle | Target | Included Features | Discount |
|--------|--------|-------------------|----------|
| **Restaurant Starter** | Cafes, warung | POS + KDS + basic inventory | 10% off vs à la carte |
| **Retail Pro** | Toko, minimarket | POS + advanced inventory + loyalty | 10% off vs à la carte |
| **Service Business** | Salons, repair shops | POS + staff management + receipts | 10% off vs à la carte |

- Available as add-ons within existing tiers
- Custom bundles for Enterprise customers

---

## 6. In-App Upgrade Triggers

> Upgrade prompts must fire at the moment of felt need — not as generic banners.
> Each trigger has a specific condition and a specific message.

### Free → Plus Triggers
| Condition | Trigger message |
| :--- | :--- |
| User views sales history older than 30 days | Blurred preview: *"Lihat riwayat lebih dari 30 hari — upgrade ke Plus"* |
| User attempts to set up QRIS | *"Aktifkan QRIS — terima pembayaran digital. Upgrade ke Plus"* |
| Second staff member tries to log in | *"Tambah anggota tim — upgrade ke Plus, hingga 5 staff"* |

### Plus → Pro Triggers
| Condition | Trigger message |
| :--- | :--- |
| User opens analytics/reports tab (locked) | Locked screen with sample chart: *"Lihat laporan lengkap — upgrade ke Pro"* |
| User attempts to add a second store | *"Buka toko ke-2 — upgrade ke Pro"* |
| Terminal count reaches 2 (Plus limit) | *"Butuh lebih banyak kasir? Pro mendukung hingga 5 terminal per toko"* |

### Pro → Premium Triggers
| Condition | Trigger message |
| :--- | :--- |
| Store count reaches 2 (approaching Pro limit) | *"Buka toko ke-3? Upgrade ke Premium — unlimited stores"* |
| Staff count reaches 16+ (approaching 20 limit) | *"Tim Anda berkembang! Premium mendukung unlimited staff"* |
| User views loyalty module (locked teaser) | Animated loyalty dashboard preview: *"Hadirkan program poin — upgrade ke Premium"* |

---

## 7. Churn Prevention

### Churn Risk by Tier

| Tier | Churn risk | Primary reason | Key intervention |
| :--- | :---: | :--- | :--- |
| **Plus** | 🔴 High | "Doesn't do enough" or post-Pro-trial letdown | Strong Daily Sales Dashboard onboarding, 30-day history trigger |
| **Pro** | 🟡 Medium | Staff/store limit reached without prompt | Proactive usage alerts at 80% of limits |
| **Premium** | 🟢 Low | Occasional downgrade to Pro | Enterprise self-serve pathway |
| **Enterprise** | 🟢 Very Low | Long contracts | Quarterly business reviews |

### Features to Implement
- **Pause subscription:** Allow 1-3 month pause (retain data, no billing) — expected -20% churn
- **Win-back campaigns:** Automated emails with special offers for churned users
- **Usage monitoring:** Alert at 80% of limits (stores, staff, terminals) — before the wall, not at it
- **Feedback collection:** Exit survey to understand churn reasons

### Metrics to Track
- Monthly churn rate by tier
- Upgrade/downgrade ratios
- Trial-to-paid conversion rate (target: 25%+)
- Net Promoter Score (NPS) by tier

---

## 8. Where each definition lives (source map)

| File | Role | Tier source |
| :--- | :--- | :--- |
| `docs/BUSINESS_PLAN.md` §2 | Market/pricing plan (IDR, annual) | 1-Time / Standard / Pro / Enterprise — **superseded banner added 2026-08-17**; content kept as historical analysis |
| `docs/decisions/2026-07-10-subscription-tier-entitlement.md` (ADR #5) | Design intent | Free / Pro / Premium / Enterprise with numeric quotas — **supersession note added 2026-08-17** (mechanism still valid; quotas from §3) |
| `docs/decisions/2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md` (ADR #23) | Trial lifecycle | 90-day trial — **re-scope note added 2026-08-17; segmented trials implemented 2026-08-18** (C2.1: `trial_vertical` in `activate.go`, 14-day Plus general / 14-day Pro restaurant-cafe / 30-day Pro enterprise-referral) |
| `website/src/content/docs/{en,id}/{licensing,welcome,installation,activation}.md` | User-facing docs | 90-day / four-tier copy — **updated to the 5-tier free-forever model 2026-08-17** |
| `crates/oz-core/src/subscription.rs` | Enforcement (client-side quotas) | enum Free/OneTime/Standard/Pro/Premium/Enterprise |
| `apps/license-server/paddle_webhook.go` → `tierQuotas()` | Enforcement (license mint) | pro/premium/enterprise → 0/0/all types; free → 1/1/3 types |
| `apps/license-server/pb_schema.json` | Schema select values | free, pro, premium, enterprise — **needs plus** |
| `apps/license-server/renew.go` | Offline renewal expiry | free +100y, pro/premium +1y, enterprise +3y |
| `website/src/content/pricing/{en,id}.ts` | Live pricing pages | **DONE (2026-08-17)** — free/plus/pro/premium/enterprise, new USD & IDR prices, annual default + "2 months free", ⭐ Pro badge, full §3 matrix |
| `apps/license-server` `PADDLE_PRICE_TIERS` (env) | Live billing map | 2 prices today: `pri_…racp:pro`, `pri_…8cec:premium` — **needs 6 once D2 lands** (monthly + yearly × Plus/Pro/Premium) |

---

## 9. Implementation Priorities

### Pre-Launch (Critical — before any paid marketing spend)
1. **Enforce staff user limits** — prevent revenue leakage
2. **Enforce 30-day sales history cap on Free** — primary upgrade forcing function
3. **Set annual plan as default on pricing page** — increases annual commit rate
4. **Add ⭐ Most Popular badge to Pro** on pricing page — anchors ARPU at the target tier
5. **Reframe annual discount as "2 bulan gratis" / "2 months free"** everywhere — stronger psychology than a percentage
6. **Build Daily Sales Dashboard as the hero feature of Plus** — lead all Plus marketing with this
7. **Define Enterprise pricing guidance** — enable sales team

### Short-Term (Month 1-3)
8. **Implement segmented trial strategy** — 14-day Plus trial for general signup; 14-day Pro trial for restaurant/cafe vertical
9. **Build vertical landing pages** — `/untuk-kafe`, `/untuk-warung`, `/untuk-minimarket`, `/untuk-restoran`
10. **Implement in-app upgrade triggers** — 80% usage limit alerts + locked feature teasers (see §6)
11. **Implement upgrade/downgrade proration** — reduce upgrade friction

### Medium-Term (Month 3-6)
12. **Phase 2: Midtrans QRIS subscriptions** — critical revenue unlock for the full Indonesian MSME market
13. **Build trial-to-paid email flows** — personalized per vertical (see §4)
14. **Create vertical-specific bundles** — 10% discount vs à la carte (see §5)
15. **Implement pause subscription feature** — reduce Plus churn

### Long-Term (Month 6-12)
16. **A/B test Pro at $7.99 vs $9.99** — optimize conversion-to-revenue balance
17. **Enterprise self-serve trial / Premium store-limit bridge** — reduce sales cycle length
18. **Launch add-on marketplace** — diversify revenue
19. **Build churn prevention automation** — improve retention

---

## 10. Success Metrics

### Revenue Metrics
- **Monthly Recurring Revenue (MRR)** growth rate
- **Average Revenue Per User (ARPU)** by tier
- **Customer Lifetime Value (LTV)** by tier
- **LTV:CAC ratio** (target: 3:1 or higher)

### Conversion Metrics
- **Free → Plus conversion rate** (target: 15%+)
- **Plus → Pro conversion rate** (target: 10%+)
- **Pro → Premium conversion rate** (target: 5%+)
- **Trial → Paid conversion rate** (target: 25%+)

### Retention Metrics
- **Monthly churn rate** (target: <5% for paid tiers)
- **Net Revenue Retention** (target: >100%)
- **Feature adoption rate** by tier
- **Support ticket volume** by tier

---

## 11. Economics Review & Financial Analysis

### Unit Economics

#### Customer Acquisition Cost (CAC) Targets
| Tier | Target CAC | CAC Payback Period |
| :--- | :---: | :---: |
| Plus | $20-30 | 4-6 months |
| Pro | $40-60 | 4-6 months |
| Premium | $80-120 | 4-6 months |
| Enterprise | $200-500 | 2-4 months |

**CAC:LTV Ratios:**
- Plus: 1:5 (LTV $150+)
- Pro: 1:6 (LTV $300+)
- Premium: 1:7 (LTV $700+)
- Enterprise: 1:10 (LTV $2,000+)

#### Average Revenue Per User (ARPU) Projections
| Tier | Monthly ARPU (USD) | Annual ARPU (USD) |
| :--- | :---: | :---: |
| Plus | $4.50 (90% monthly) | $54.00 |
| Pro | $9.00 (90% monthly) | $108.00 |
| Premium | $18.00 (90% monthly) | $216.00 |
| Enterprise | $250.00 | $3,000.00 |

**Blended ARPU Target:** $12-15/month (across all paid tiers)

#### Customer Lifetime Value (LTV) by Tier
| Tier | Avg. Lifespan | LTV (USD) |
| :--- | :---: | :---: |
| Plus | 12 months | $54 |
| Pro | 18 months | $162 |
| Premium | 24 months | $432 |
| Enterprise | 36 months | $9,000 |

**Assumptions:**
- Monthly churn rates: Plus 8%, Pro 5%, Premium 4%, Enterprise 2.8%
- No price increases assumed
- No expansion revenue (upsells) included

### Revenue Projections (Year 1-3)

#### Scenario: Conservative Growth
| Year | Total Customers | Paid Customers | MRR (USD) | ARR (USD) |
| :--- | :---: | :---: | :---: | :---: |
| Year 1 | 10,000 | 1,500 | $18,000 | $216,000 |
| Year 2 | 25,000 | 4,000 | $48,000 | $576,000 |
| Year 3 | 50,000 | 8,000 | $96,000 | $1,152,000 |

#### Scenario: Moderate Growth
| Year | Total Customers | Paid Customers | MRR (USD) | ARR (USD) |
| :--- | :---: | :---: | :---: | :---: |
| Year 1 | 15,000 | 2,500 | $30,000 | $360,000 |
| Year 2 | 40,000 | 7,000 | $84,000 | $1,008,000 |
| Year 3 | 80,000 | 15,000 | $180,000 | $2,160,000 |

#### Scenario: Aggressive Growth
| Year | Total Customers | Paid Customers | MRR (USD) | ARR (USD) |
| :--- | :---: | :---: | :---: | :---: |
| Year 1 | 20,000 | 4,000 | $48,000 | $576,000 |
| Year 2 | 60,000 | 12,000 | $144,000 | $1,728,000 |
| Year 3 | 120,000 | 25,000 | $300,000 | $3,600,000 |

**Key Assumptions:**
- Free → Paid conversion: 15% (Year 1), 18% (Year 2), 20% (Year 3)
- Monthly churn improves: 8% → 6% → 5% (paid tiers)
- 10% of customers upgrade tiers annually
- Enterprise represents 5% of paid customers but 25% of revenue

### Cost Structure Analysis

#### Fixed Costs (Monthly)
| Category | Cost (USD) | Notes |
| :--- | :---: | :--- |
| Cloud Infrastructure | $2,000-5,000 | Scales with customers |
| Development Team | $15,000-25,000 | 3-5 engineers |
| Support Team | $3,000-6,000 | 1-2 support staff |
| Office/Admin | $1,000-2,000 | Minimal remote setup |
| **Total Fixed** | **$21,000-38,000** | |

#### Variable Costs (Per Customer)
| Category | Cost (USD) | Notes |
| :--- | :---: | :--- |
| Payment Processing | 3-5% of revenue | Paddle/Midtrans fees |
| Cloud Storage | $0.10-0.50/mo | PostgreSQL + backups |
| Support | $0.50-2.00/mo | Email/chat tickets |
| **Total Variable** | **4-8% of revenue** | |

#### Break-Even Analysis
**Fixed Costs:** $30,000/month (midpoint)
**Variable Costs:** 6% of revenue
**Required Revenue:** $31,915/month (to cover fixed + variable)
**Required Paid Customers:** ~2,100 (at $15 ARPU)

**Break-Even Timeline:**
- Conservative: Month 18-24
- Moderate: Month 12-15
- Aggressive: Month 8-10

### Market Sizing (Indonesia)

#### Total Addressable Market (TAM)
- **Indonesian MSMEs:** ~65 million businesses
- **Retail/Hospitality Focus:** ~15 million businesses
- **POS System Adoption:** ~30% (4.5 million businesses)
- **Digital POS Adoption:** ~10% (1.5 million businesses)

#### Serviceable Addressable Market (SAM)
- **Target Segment:** Digital-ready SMEs in urban areas
- **Estimate:** 500,000 - 1,000,000 businesses
- **Annual Spending:** $100-500/year on POS systems

#### Serviceable Obtainable Market (SOM)
- **Year 1 Target:** 0.1-0.3% of SAM
- **Estimate:** 500 - 3,000 paying customers
- **Revenue Potential:** $50,000 - $300,000 ARR

### Financial Risks & Mitigations

#### High-Risk Factors
| Risk | Impact | Probability | Mitigation |
| :--- | :---: | :---: | :--- |
| Low conversion from Free to Paid | High | Medium | Optimize trial flow, add daily sales summary |
| High churn in Plus tier | High | High | Improve onboarding, add value features |
| Enterprise sales cycle too long | Medium | Medium | Create self-serve Enterprise trial |
| Currency fluctuation (IDR/USD) | Medium | High | Quarterly price reviews, hedging |

#### Revenue Leakage Risks
| Risk | Impact | Mitigation |
| :--- | :---: | :--- |
| Staff user limits not enforced | High | Implement before launch (CRITICAL) |
| Offline grace period abuse | Medium | Reduce Free tier to 7 days |
| Downgrade without data export | Medium | 14-day grace period implemented |

### Optimization Opportunities

#### Pricing Optimization
1. **A/B Test Pro Tier:** Test $7.99 vs $9.99 for Pro tier
   - Expected impact: +10-15% Pro adoption
   - Risk: -5-10% revenue per Pro customer

2. **Add Usage-Based Component:** 1% fee on QRIS transactions for Plus/Pro
   - Expected impact: +$2-5 ARPU for active users
   - Risk: Increased complexity, customer resistance

3. **Bundle Discounts:** 10-15% for vertical-specific bundles
   - Expected impact: +20% bundle adoption
   - Risk: Cannibalization of individual tier sales

#### Conversion Optimization
1. **Free Tier Expansion:** Add 2 terminals and 3 staff for 30 days
   - Expected impact: +25% trial activation
   - Risk: Increased support costs

2. **Upgrade Prompts:** In-app prompts at usage milestones
   - Expected impact: +15% upgrade rate
   - Risk: User annoyance if overdone

3. **Success Stories:** Case studies from similar businesses
   - Expected impact: +10% conversion
   - Risk: Content creation cost

#### Retention Optimization
1. **Pause Subscription:** 1-3 month pause option
   - Expected impact: -20% churn
   - Risk: Delayed revenue recognition

2. **Win-Back Campaigns:** Automated emails with offers
   - Expected impact: +5% win-back rate
   - Risk: Discount fatigue

3. **Usage Monitoring:** Alerts before tier limits
   - Expected impact: -15% involuntary churn
   - Risk: Development cost

### Key Financial Metrics to Track

#### Weekly Metrics
- Trial activation rate
- Free → Paid conversion (daily)
- Support ticket volume
- Payment failure rate

#### Monthly Metrics
- MRR growth rate
- ARPU by tier
- Churn rate by tier
- CAC by acquisition channel
- LTV:CAC ratio

#### Quarterly Metrics
- Net Revenue Retention
- Expansion revenue %
- Customer satisfaction (NPS)
- Feature adoption rates

### Financial Milestones

#### Year 1 Targets
- **Q1:** 500 paying customers, $6,000 MRR
- **Q2:** 1,000 paying customers, $12,000 MRR
- **Q3:** 1,500 paying customers, $18,000 MRR
- **Q4:** 2,000 paying customers, $24,000 MRR

#### Year 2 Targets
- **Q1:** 3,000 paying customers, $36,000 MRR
- **Q2:** 4,500 paying customers, $54,000 MRR
- **Q3:** 6,000 paying customers, $72,000 MRR
- **Q4:** 8,000 paying customers, $96,000 MRR

#### Year 3 Targets
- **Q1:** 10,000 paying customers, $120,000 MRR
- **Q2:** 13,000 paying customers, $156,000 MRR
- **Q3:** 16,000 paying customers, $192,000 MRR
- **Q4:** 20,000 paying customers, $240,000 MRR

### Investment Requirements

#### Seed Round (Pre-Launch)
- **Amount:** $100,000 - $200,000
- **Use:** Development completion, initial marketing
- **Timeline:** 6-12 months runway

#### Series A (Year 1-2)
- **Amount:** $500,000 - $1,000,000
- **Use:** Team expansion, market expansion, sales
- **Timeline:** 18-24 months runway

#### Key Metrics for Fundraising
- **MRR Growth:** 15-20% month-over-month
- **CAC Payback:** <6 months
- **LTV:CAC:** >3:1
- **Churn Rate:** <5% monthly (paid tiers)

### Economic Recommendations Summary

#### Immediate Actions (Pre-Launch)
1. **Enforce staff user limits** — prevent revenue leakage
2. **Implement daily sales summary in Plus** — improve conversion
3. **Define Enterprise pricing ranges** — enable sales

#### Short-Term Optimizations (0-3 months)
1. **A/B test Pro tier pricing** — optimize revenue
2. **Launch 30-day Pro trial** — boost conversion
3. **Implement upgrade prompts** — increase adoption

#### Medium-Term Strategies (3-6 months)
1. **Add usage-based pricing component** — diversify revenue
2. **Create vertical bundles** — expand market
3. **Implement churn prevention features** — improve retention

#### Long-Term Growth (6-12 months)
1. **Launch add-on marketplace** — increase ARPU
2. **Expand to adjacent markets** — geographic growth
3. **Build enterprise self-serve** — reduce sales cycle

---

## Appendix: Financial Formulas

### Unit Economics Calculations

#### Customer Lifetime Value (LTV)
```
LTV = ARPU × (1 / Churn Rate)
Example (Plus): $4.50 × (1 / 0.08) = $56.25
```

#### Customer Acquisition Cost (CAC)
```
CAC = Total Sales & Marketing Spend / New Customers Acquired
Target: LTV / CAC ≥ 3:1
```

#### Payback Period
```
Payback Period = CAC / ARPU
Example (Plus): $25 / $4.50 = 5.6 months
```

#### Break-Even Point
```
Break-Even Customers = Fixed Costs / (ARPU × (1 - Variable Cost %))
Example: $30,000 / ($15 × 0.94) = 2,128 customers
```

#### Monthly Recurring Revenue (MRR)
```
MRR = Σ (Monthly Revenue from All Customers)
ARR = MRR × 12
```

#### Net Revenue Retention (NRR)
```
NRR = (Starting MRR + Expansion - Contraction - Churn) / Starting MRR
Target: >100% (growth from existing customers)
```

### Sensitivity Analysis

#### Impact of Churn Rate Changes on LTV
| Churn Rate | Plus LTV | Pro LTV | Premium LTV |
| :---: | :---: | :---: | :---: |
| 10% | $45 | $90 | $180 |
| 8% | $56 | $112 | $225 |
| 6% | $75 | $150 | $300 |
| 4% | $112 | $225 | $450 |
| 2% | $225 | $450 | $900 |

#### Impact of ARPU Changes on Break-Even
| ARPU | Customers Needed | Time to Break-Even |
| :---: | :---: | :---: |
| $10 | 3,192 | 24 months |
| $12 | 2,660 | 20 months |
| $15 | 2,128 | 16 months |
| $18 | 1,773 | 13 months |
| $20 | 1,596 | 12 months |

### Financial Model Assumptions

#### Growth Assumptions
- **Market Growth:** 15-20% annually (Indonesian POS market)
- **Competitive Landscape:** 5-10 major competitors
- **Technology Adoption:** Accelerating post-pandemic
- **Regulatory Environment:** Supportive for digital payments

#### Cost Assumptions
- **Cloud Costs:** Decreasing 10-15% annually
- **Payment Processing:** Stable at 3-5%
- **Labor Costs:** Increasing 5-8% annually
- **Marketing Costs:** Stable as percentage of revenue

#### Revenue Assumptions
- **Pricing Power:** Moderate (price-sensitive market)
- **Expansion Revenue:** 10-15% annually from existing customers
- **Enterprise Mix:** 5% of customers, 25% of revenue
- **Geographic Mix:** 70% Indonesia, 30% global
