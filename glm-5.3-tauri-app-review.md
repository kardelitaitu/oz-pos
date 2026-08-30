# GLM-5.3 Tauri App Review — Journal

> Reviewer: GLM-5.3 (DeepSeek Harness agent)
> Started: 2026-07-25 · Branch: `0.0.33` (locked — never switch)
> Scope: OZ-POS Tauri application end-to-end review (desktop + tablet), its supporting
> crates, platform layer, modules, UI, and adjacent cloud/licensing surfaces.
>
> Rules of engagement:
> - Commit only this agent's own work; stage files explicitly (never `git add -A`).
> - No push. No branch changes.
> - Use codebase-memory-mcp (project `oz-pos`) for structure/relationship queries.
> - Read files in ≤500-line chunks.
> - Version stays locked at `0.0.33`.

---

## How to use this journal

- **Sector map** below defines the review areas. Each sector gets a status line
  (`not-started` / `in-progress` / `reviewed` / `follow-up`) and a notes section.
- **Findings log** records concrete issues with a stable finding ID, severity
  (`P0 critical` / `P1 major` / `P2 minor` / `P3 nit` / `INFO`), and file references.
- Append session notes at **Session log** (date, sectors touched, commits).
- A sector is `reviewed` only when every file in its scope has been either read or
  explicitly dispositioned (e.g. "generated file, skip").

---

## S0 — Orientation & Baseline

**Status:** reviewed (2026-07-25)

Repo shape (from codebase-memory graph, 40,314 nodes / 204,452 edges):

| Area | Members |
|------|---------|
| `apps/` | `desktop-client`, `tablet-client`, `cloud-server`, `license-server` (Go), `unified` |
| `crates/` | `oz-api`, `oz-cli`, `oz-core`, `oz-crypto`, `oz-hal`, `oz-logging`, `oz-lua`, `oz-media`, `oz-notification`, `oz-payment`, `oz-plugin`, `oz-reporting`, `oz-security` |
| `modules/` | 14 business modules: `crm`, `currency`, `giftcards`, `inventory`, `kitchen`, `loyalty`, `promotions`, `purchasing`, `reporting`, `sales`, `settings`, `staff`, `tax`, `terminal` |
| `platform/` | `core` (auth/rbac/settings/terminal_profile), `kernel` (event_bus, manifest), `startup`, `sync` |
| `foundation/` | shared foundation crate (barcode, validation, updates) |
| `ui/` | React 18 + TypeScript front-end (2 bundles: desktop `index.html`, tablet `index.tablet.html`) |
| `website/` | Astro marketing site + Cloudflare Worker (`worker.ts`) |
| Tooling | `scripts/` (~100 files), `.github/` CI, `packaging/`, 5× docker-compose variants |

Languages: TypeScript (901 files), Rust (819), Go (52), CSS (125), Python (28), Kotlin (12).

Graph hotspots (highest fan-in — inspect first, blast-radius risk):
1. `oz-core::db::Store::new` — fan-in 1312
2. `license-server::lock` (Go) — 1262
3. `oz-payment::drivers::qris::QrisPaymentProcessor::clone` — 731
4. `oz-plugin::db::PluginDb::execute` — 554
5. `foundation::barcode::Barcode::from_str` — 478
6. `ui::utils::logged-invoke::loggedInvoke` — 477
7. `desktop-client::state::AppState::drop` — 461
8. `platform::startup::event_handlers::fresh_db` — 268
9. `tablet-client::state::AppState::resolve_scope` — 266
10. `oz-core::migrations::fresh_db` — 256

Pre-existing dirty tree at review start (NOT mine, do not commit):
`apps/license-server/{midtrans_webhook.go,paddle_webhook.go,pb_schema.json,revenue_events.go}`,
`docs/specs/revenue-data-pipeline-plan.md`, `pr_body.md`, `website/package{,-lock}.json`,
`glm5.3f-crates-audit.md`, `scratch/`.

### S0 Notes

- **Workspace members** (`Cargo.toml`): `crates/*`, `modules/*`, `platform/*`, `foundation`,
  `apps/cloud-server`, `apps/desktop-client`, `apps/tablet-client`.
- **Scale**: Rust ≈ 246k LoC (819 files; 316 sibling `*_tests.rs` files), `ui/src` ≈ 199k
  LoC TS/TSX, 379 UI test files, 31 Playwright e2e specs, ~100 scripts, 11 CI workflows.
- **CI workflows**: `ci.yml`, `e2e-pr.yml`, `nightly.yml`, `security.yml`, `release.yml`,
  `website.yml`, `deploy.yml`, `android.yml`, `ios.yml`, `docker-digest-drift.yml`,
  `docker-persistence.yml`. Trigger topology to be verified in S9 (AGENTS.md says CI
  runs only on main; mobile/website/docker flows may differ).
- **Methodology declaration**: with 2,551 files, this review is (a) structural sweeps via
  codebase-memory graph, (b) automated invariant checks (grep/pattern), and (c) targeted
  deep-dives on graph hotspots + entry points. It is not a line-by-line read of every
  file; sector notes state exactly what was checked.
- Dirty worktree files present at review start are listed above and excluded from all
  review commits.

### S0 Findings

- **F-001 (INFO, S0)**: `apps/unified/` is a Docker deployment bundle (Caddyfile,
  supervisord.conf, healthcheck scripts), deliberately excluded from the Cargo workspace
  per `Cargo.toml:12` comment — by design, no action.
- **F-002 (INFO, S0)**: CI surface (11 workflows incl. mobile/website/docker) is broader
  than the AGENTS.md "CI only triggers on main" summary — S9 verifies each trigger.
- **F-003 (INFO, S0)**: review scale requires sampling strategy (see methodology above).

---

## S1 — Tauri Shells & IPC Surface

**Status:** not-started

**Scope:** `apps/desktop-client/`, `apps/tablet-client/` (Tauri v2), incl.
`src/commands/`, `src/state/`, `lib.rs` registration, `tauri.conf.json`,
capabilities/permissions; the IPC boundary `ui/src/api/*` + `ui/src/utils/logged-invoke.ts`
+ `ui/src/dev-mock/`.

**Key questions**
- Are all commands defined under `src/commands/` and registered in `lib.rs`? Any orphan or double-registered?
- Is `invoke()` never called directly from components (AGENTS rule) — is `loggedInvoke` the single choke point?
- Desktop vs tablet divergence: how much command surface is duplicated between the two clients? Drift risk.
- `AppState` lifecycle (hotspot: `drop` at fan-in 461; tablet `resolve_scope` 266) — leak/teardown correctness.
- Capability/permission scope in tauri.conf — least privilege?

### S1 Notes

- (empty)

---

## S2 — Data Core (oz-core)

**Status:** not-started

**Scope:** `crates/oz-core/` — `db::Store` (hotspot, fan-in 1312), migrations,
Money/i64 representation, schema.

**Key questions**
- All monetary values `i64` minor units via `Money` struct — zero `f32`/`f64` currency leakage.
- Every DB write inside a `rusqlite` transaction; no stray `execute` outside tx.
- Migration hygiene: ordering, idempotency, `fresh_db` test path parity with prod path.
- Store concurrency model (single conn? pool? mutex?) — hotspot fan-in suggests everything touches it.

### S2 Notes

- (empty)

---

## S3 — Platform Layer (core / kernel / startup / sync)

**Status:** not-started

**Scope:** `platform/core/` (auth.rs, rbac.rs, permission_registry.rs, settings/,
terminal_profile.rs), `platform/kernel/` (event_bus, manifest), `platform/startup/`
(event_handlers, rate_sync, metrics), `platform/sync/` (queue, daemon, pg_daemon,
conflict, replication, transport).

**Key questions**
- RBAC + permission registry: complete coverage of command surface in S1? Any command callable by wrong role?
- Event bus ordering/lifecycle; kernel module manifest validation (`platform/kernel/tests/module_manifests.rs`).
- Startup `fresh_db` path (fan-in 268) — does dev path equal shipped path?
- Sync: conflict resolution correctness, offline queue durability, PG vs SQLite transport parity.

### S3 Notes

- (empty)

---

## S4 — Business Modules (modules/*)

**Status:** not-started

**Scope:** 14 modules under `modules/`, each with `manifest.json`, `models.rs`,
`repository.rs`, `service.rs`, `error.rs`, sibling `*_tests.rs`.

**Key questions**
- Uniform structure per module (error.rs thiserror, service/repo split, boundary contract tests like `modules/tax/tests/boundary_contract.rs`).
- Manifest ↔ registration parity (kernel manifest test).
- Money handling and tx discipline inside each service.
- `sales` + `tax` + `promotions` interplay — pricing correctness (order of discounts/tax).
- Module fan-in boundaries from graph: `crm`→kernel/oz-payment edges look suspicious, verify.

### S4 Notes

- (empty)

---

## S5 — Payments, Hardware & Security Crates

**Status:** not-started

**Scope:** `crates/oz-payment/` (drivers incl. `qris`, hotspot `QrisPaymentProcessor::clone` 731),
`crates/oz-hal/` (embedded-hal drivers + `drivers/mock.rs`), `crates/oz-crypto/`,
`crates/oz-security/`.

**Key questions**
- Payment state machine: can a payment double-fire / replay? QRIS callback validation.
- `clone` on payment processor with fan-in 731 — shared state aliasing bugs?
- HAL: every real driver has mock; feature-gating so desktop builds don't link device code.
- oz-crypto/oz-security: key storage, no homegrown crypto, no secrets in repo.

### S5 Notes

- (empty)

---

## S6 — Front-end UI (ui/)

**Status:** not-started

**Scope:** `ui/src/features/**` (33 feature dirs), `components/`, `contexts/`, `hooks/`,
`i18n/` + `locales/` (FTL bundles), `utils/` (chart-policy, list-policy, currency),
2 entry bundles (desktop/tablet), 379 test files in `__tests__/`, 31 e2e specs.

**Key questions**
- All user-visible strings via `@fluent/react` (`<Localized id>`); bundle parity gate holds.
- ARIA labels everywhere; eslint-plugin-jsx-a11y clean.
- Contexts (`AuthContext`, `SettingsContext`, `WorkspaceContext`, `CurrencyContext`…) — re-render blast radius; provider ordering in `AppProviders.tsx`.
- `loggedInvoke` (fan-in 477) — error mapping consistency to UI.
- Desktop/tablet shared component strategy — copy-paste drift (RetailPosScreen vs PosScreen?).

### S6 Notes

- (empty)

---

## S7 — Auxiliary Crates

**Status:** not-started

**Scope:** `crates/oz-api`, `oz-cli`, `oz-lua` (plugin scripting — `scripts/examples/*.lua`,
`plugins/example-discount/`), `oz-media`, `oz-notification`, `oz-reporting`, `oz-logging`,
`oz-plugin` (`PluginDb::execute` hotspot 554), `foundation/`.

**Key questions**
- oz-lua sandboxing: can a plugin escape (fs/network/unsafe)? Resource limits?
- oz-plugin `PluginDb::execute` — SQL built from plugin data? Injection surface.
- oz-reporting boundary (`oz-reporting→oz-core/oz-payment` edges in graph) — layering violations.
- foundation barcode parse (`from_str` fan-in 478) — malformed input handling.

### S7 Notes

- (empty)

---

## S8 — Cloud, Licensing & Web

**Status:** not-started

**Scope:** `apps/cloud-server/` (PG, RLS, tenant keys), `apps/license-server/` (Go —
Paddle/Midtrans/Square/Stripe webhooks; note: webhook files currently dirty in worktree),
`apps/unified/`, `website/worker.ts` (auth, OTP, checkout), `website/src/` (Astro).

**Key questions**
- Webhook signature verification per provider (license-server Go); idempotent processing.
- RLS policies vs `scripts/rls-cutover.sql` vs committed `PG_INIT` — drift (see AGENTS dev-PG drift note).
- Worker auth: session handling, password policy (`website/scripts/check-password-policy.mjs`), OTP flow.
- Revenue events pipeline (new, uncommitted files present) — mark as out-of-scope unless asked.

### S8 Notes

- (empty)

---

## S9 — Build, CI & Tooling

**Status:** not-started

**Scope:** `scripts/` (~100), `.github/workflows/`, 4 pre-commit gates (fmt, i18n lint,
bundle-parity, FTL dedupe) in `.githooks/`, `deny.toml`, `rust-toolchain.toml`,
docker-compose matrix, `packaging/`, coverage/flaky-quarantine infra.

**Key questions**
- Gate coverage: can any AGENTS invariant (Money, no-raw-params, hardcoded money format)
  be committed without a gate catching it?
- CI only on main (push+PR) — feature branches unvalidated; is `check.sh` parity with CI real?
- verify-architecture-boundaries baseline (`scripts/architecture-boundaries-baseline.json`) — stale allowances?

### S9 Notes

- (empty)

---

## S10 — Docs & Drift

**Status:** not-started

**Scope:** `ARCHITECTURE.md`, `docs/` (incl. `docs/specs/`), `JOURNAL.md`, `TODO.md`,
`CHANGELOG.md`, `.agents/skills/**`, `documentation.md`, root-level plan docs.

**Key questions**
- Do docs match reality (docs-auditor style audit, truth anchors)?
- Spec files in `docs/specs/_active/` — implemented vs abandoned?
- Skill drift: `skill-drift-report.md` findings vs current code.

### S10 Notes

- (empty)

---

## Cross-cutting review criteria (apply to every sector)

1. **Money:** `i64` minor units via `Money`; no `f32`/`f64` currency anywhere.
2. **DB writes:** always inside `rusqlite` transactions (PG: tokio_postgres tx).
3. **Errors:** `thiserror` for types, `anyhow` for app-level propagation; no `unwrap`/`expect` in prod paths (see `scripts/scan-unwrap-panic.py`).
4. **Doc comments:** `///` on every public item; `//!` module header 5–15 lines per prod `.rs` file.
5. **File size:** production `.rs` < 1000 lines (prefer < 600); tests in sibling `*_tests.rs`, integration in `tests/`.
6. **i18n:** no hardcoded English strings in JSX; FTL keys present in both bundles.
7. **A11y:** ARIA labels; jsx-a11y clean.
8. **IPC:** UI → `ui/src/api/*` only; no direct `invoke` in components.
9. **Version lock:** `0.0.33` everywhere (Cargo.toml, tauri.conf.json, package.json, CHANGELOG).
10. **Secrets:** no secrets/`.env`/SQLite DB files committed.

---

## Findings log

| ID | Date | Sector | Severity | Location | Finding |
|----|------|--------|----------|----------|---------|
| F-001 | 2026-07-25 | S0 | INFO | `apps/unified/`, `Cargo.toml:12` | Docker deployment bundle, deliberately outside workspace — by design |
| F-002 | 2026-07-25 | S0 | INFO | `.github/workflows/` | 11 workflows, broader than AGENTS.md "CI only on main" summary — verify in S9 |
| F-003 | 2026-07-25 | S0 | INFO | repo-wide | 2,551 files → sampling methodology (structural + invariant checks + hotspot deep-dives) |

---

## Session log

### 2026-07-25 — Session 1
- Created this journal; mapped repo via codebase-memory graph (architecture overview +
  file tree + crate/app/module inventory).
- Recorded graph hotspots (S0) and pre-existing dirty worktree files (excluded from my commits).
- **S0 reviewed**: workspace members, scale stats (246k Rust LoC / 199k TS LoC), 11 CI
  workflows, methodology declaration, findings F-001..F-003. Committed.
- Next: S1 Tauri shells & IPC.
