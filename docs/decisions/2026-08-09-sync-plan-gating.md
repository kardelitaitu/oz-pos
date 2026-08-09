# ADR: Gate cloud sync behind a paid plan

**Date:** 2026-08-09 · **Status:** In progress (E1–E3 done) · **Owner:** buffy

## Problem

Cloud sync is a paid feature: a tenant on the `free` plan may run the POS
fully locally (sales, offline queue writes, KDS, topology) but must not be
able to push/pull to the cloud server. There was no plan concept anywhere
in the stack, and nothing stopped a free tenant from syncing.

## Decision

Enforce the gate **server-side**, keyed off the **`tenant_id` in the JWT
claims** — never trust the client to self-report its plan, and never take
the tenant from the request body (tenant spoofing).

- **Plan state** lives in a new `tenant_plans` table keyed by `tenant_id`
  (migration `126`). Plans are per-store, not per-terminal: every terminal
  of a store inherits the store's plan (`sync_terminals.tenant_id` joins to
  it). Values: `free` | `pro`, validated by a `CHECK` constraint.
- **Enforcement** is a `plan_middleware` on the sync router, layered
  between `auth_middleware` (provides claims) and the handlers — the same
  position as `rate_limit_middleware`. Free tenants get a structured
  `403 {"error":"plan_required"}`; pro tenants pass through.
- **Opt-in via `OZ_ENFORCE_PLANS=1`.** When unset (dev/local Docker)
  nothing changes — no plan row = allowed, matching the `OZ_ADMIN_KEY`
  "open when unset" pattern. When set (production), fail closed: a missing
  row is treated as `free`.
- **Admin endpoint** `PUT /api/v1/tenants/{tenant_id}/plan` sets a plan,
  gated by the same `OZ_ADMIN_KEY` as token minting. Stripe webhook
  scaffolding already exists in `webhooks.rs` for a future billing hookup.
- Only `/api/sync/*` is gated. Local POS remains fully functional on free.

### Client behaviour (E3)

A `plan_required` response must be **terminal**: no token refresh, no
retry spin, and critically **no quarantine** — queued items stay `pending`
(they are valid, just unsendable on a free plan). The daemon backs off and
surfaces the reason; the UI shows "Sync requires a paid plan" rather than
"disconnected" (E4).

Implemented: `SyncError::PlanRequired` (platform-sync) and
`SyncHttpError::PlanRequired` (oz-core) are classified from a structured
`403 {"error":"plan_required"}` in push/pull/snapshot, and the daemon
treats the variant as terminal — no refresh (the refresh path matches only
`AuthExpired`), no in-tick retry, and queued items stay `pending`. A
daemon regression test asserts the error surfaces, the item is untouched,
and each endpoint is hit exactly once per tick.

## Alternatives considered

- **402 Payment Required** instead of 403: 403 keeps consistency with the
  existing client error classification; 402 adds no benefit for a
  self-hosted server. Chose 403.
- **Client-side gating**: rejected — trivially bypassable, and the client
  is not the source of truth.
- **Plan embedded in the JWT**: rejected — stale up to token expiry (24h);
  a per-request DB check is cheap (sync is ~1 req/60s per terminal) and
  takes effect immediately.

## Files

- `crates/oz-core/migrations/126_tenant_plans.sql` — table
- `crates/oz-core/src/db/plans.rs` — `TenantPlan` + `Store` get/set/list
- `crates/oz-api/src/routes/plans.rs` — admin plan endpoint
- `apps/cloud-server/src/sync_api.rs` — `plan_middleware` + tests

## Follow-ups

- E3: client `PlanRequired` classification, daemon no-retry/no-quarantine.
- E4: UI status state + upgrade CTA.
- Stripe webhook → `set_tenant_plan` wiring (scaffolding exists).
