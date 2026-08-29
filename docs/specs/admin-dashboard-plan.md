# Admin Dashboard — Feature Plan (ADR #42 Phase 3+)

**Status:** Draft — mock data phase  
**Data source:** Mock for now; real Paddle/Midtrans wiring later  
**Target:** `admin.ozpos.my.id` (existing auth-gated admin SPA)

---

## 1. KPIs (stat cards)

| KPI | Definition | Source (final) | Mock value |
|---|---|---|---|
| **Total Users** | Count of `tenants` records (all statuses) | `GET /api/v1/admin/stats` → `tenants` count | 1,247 |
| **Active Users** | `tenants.status == 'active'` | same | 1,084 |
| **Total Subscribers** | Tenants with a **non-free** subscription (plus/pro/premium/enterprise) | `subscriptions` where `tier_key != 'free'` + `status == 'active'` | 386 |
| **Monthly Gross (IDR)** | Sum of active subscriptions' monthly price → converted USD→IDR live | Paddle/Midtrans `amount` + live FX | Rp 68,4 jt (≈ $4,280 × 16,000) |
| **MRR (Monthly Recurring Revenue)** | Monthly-equivalent of active subscriptions | Paddle/Midtrans price map | $4,280 |
| **ARPU** | MRR ÷ active subscribers | computed | $11.09 |
| **Active Devices** | `tenant_machines` where `revoked_at` empty | same | 812 |
| **Trial → Paid conv.** | Trial tenants that became subscribers | `trial_claims` + `subscriptions` | 22.4% |

---

## 2. Advanced charts

| Chart | Type | X axis | Series | Data |
|---|---|---|---|---|
| **Revenue trend** | Line/area | Last 12 months | Monthly gross (USD + IDR) | Mock series, later Paddle `transaction.completed` |
| **Subscriber growth** | Area | Last 12 months | Cumulative subscribers | `subscriptions.starts_at` bucketed |
| **Tier distribution** | Donut | — | plus / pro / premium / enterprise | `subscriptions.tier_key` |
| **Signups per month** | Bar | Last 12 months | New tenants | `tenants.created` |
| **Churn / canceled** | Line | Last 12 months | Canceled per month | `subscriptions.status == 'expired'/'revoked'` |
| **Payment provider split** | Donut | — | Paddle / Midtrans | `subscriptions.payment_provider` |

---

## 3. Tables

- **Top subscribers** — tenant email, tier, MRR, next renewal, provider
- **Recent signups** — email, created, verified, current tier
- **Expiring soon** — subscriptions expiring within 30 days (renewal outreach)

---

## 4. USD → IDR live conversion

- **Live source:** `https://open.er-api.com/v6/latest/USD` (free, no key) — returns `rates.IDR`.
- **Fallback:** hardcoded last-known rate (e.g. 16,000) if the API is unreachable; the card shows `≈` and a "live rate" chip with the timestamp.
- **Cache:** 1-hour in the SPA so it doesn't hammer the FX API on every tab switch.
- **Final (Phase 3 real):** Midtrans charges fixed IDR directly (no conversion needed); Paddle charges USD — convert only Paddle revenue.

---

## 5. API contract (final shape — mock returns the same JSON)

```jsonc
// GET /api/v1/admin/stats  (OZ_ADMIN_KEY / admin tenant session)
{
  "kpis": {
    "totalUsers": 1247,
    "activeUsers": 1084,
    "totalSubscribers": 386,
    "mrrUsd": 4280,
    "mrrIdr": 68480000,
    "arpuUsd": 11.09,
    "activeDevices": 812,
    "trialToPaidRate": 22.4,
    "fxRate": 16000,
    "fxUpdatedAt": "2026-08-29T12:00:00Z"
  },
  "revenueTrend": [ { "month": "2026-09", "usd": 4120, "idr": 65920000 }, ... ],
  "subscriberGrowth": [ { "month": "2026-09", "count": 386 }, ... ],
  "signupsPerMonth": [ { "month": "2026-09", "count": 87 }, ... ],
  "churnPerMonth": [ { "month": "2026-09", "count": 12 }, ... ],
  "tierDistribution": [ { "tier": "plus", "count": 210 }, ... ],
  "providerSplit": [ { "provider": "paddle", "count": 264 }, { "provider": "midtrans", "count": 122 } ],
  "topSubscribers": [ { "email": "...", "tier": "pro", "mrrUsd": 9.99, "renewal": "...", "provider": "paddle" }, ... ],
  "recentSignups": [ { "email": "...", "created": "...", "verified": true, "tier": "free" }, ... ],
  "expiringSoon": [ { "email": "...", "tier": "plus", "expiresAt": "...", "daysLeft": 12 } ]
}
```

---

## 6. Implementation phases

1. **Mock (now):** admin SPA renders a full Dashboard tab from a `MOCK_STATS` object (this file). Live FX fetch + fallback. SVG charts (no external lib).
2. **API endpoint:** add `GET /api/v1/admin/stats` to the license server returning real aggregates from `tenants`/`subscriptions`/`tenant_machines` (counts + tier/provider splits).
3. **Revenue wiring:** persist Paddle `transaction.completed` amounts + Midtrans `gross_amount` into a `revenue_events` collection; the stats endpoint sums them.
4. **Live FX in backend:** the stats endpoint refreshes the USD→IDR rate server-side (cache 1h) so the dashboard is consistent.

---

## 7. Notes / decisions

- **Monthly gross = MRR** in this build (recurring, not one-time). One-time bundle purchases are separate if needed later.
- **IDR conversion:** only Paddle (USD) revenue converts; Midtrans is already IDR. When real data lands, sum each provider's native currency then convert.
- Mock figures are clearly labeled so operators don't mistake them for live data.
