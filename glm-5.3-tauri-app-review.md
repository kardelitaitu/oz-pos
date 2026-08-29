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

**Status:** reviewed (2026-07-25)

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

**Checked:** command registration completeness (programmatic diff of `#[tauri::command]`
definitions vs `generate_handler` entries, both shells), Tauri security config, IPC
boundary discipline (direct-invoke audit of all of `ui/src`), dead-command surface
cross-referenced against every UI call site, dev-mock fallback behavior, `AppState`
lifecycle, capabilities files.

- Scale: desktop 53 prod command files / 568 `#[tauri::command]` attrs / 374 registered
  in `generate_handler` (lib.rs 721 lines); tablet 41 files / 403 attrs / 363 registered
  (578 lines).
- **ADR #7 (multi-store scoping) migration is half-landed:** desktop registers
  `_scoped` variants only; tablet registers unscoped AND scoped for several commands
  (e.g. `settings::get_setting`/`set_setting` registered on tablet, absent on desktop).
- The UI api layer still exports ~80 wrappers calling unscoped command names; only a
  handful have live callers (all enumerated in findings).
- Dev-mock (`ui/src/dev-mock/tauri-api.ts`) delegates to real IPC inside a webview but
  answers everything in browser E2E; a `SCOPED_ALIASES` table (line ~2997) with an
  honest comment admits scoped/unscoped registration gaps.
- CSP is tight on both shells (see F-009). Desktop capability: `default.json`; tablet
  adds `mobile.json`.
- Desktop `AppState` is `Arc<Mutex<Connection>>` + `DriverRegistry` + `Option<AppHandle>`;
  `Drop` aborts plugin hot-reload task and signals kernel shutdown — orderly teardown.
  Single-connection SQLite model noted for S2.
- Topology commands flagged "registered-but-undefined" by the parser are false positives:
  `topology.rs` re-exports from a submodule; verified defined.

### S1 Findings

- **F-004 (P1, S1)**: Desktop-unregistered commands still invoked by live UI shell code —
  silent failures on desktop, working on tablet:
  - `UpdateBanner.tsx:211` → `set_setting` (updater persistence; comment says "unscoped
    invoke" deliberately, but unscoped is NOT registered on desktop; try/catch masks it)
  - `useCloudSync.ts:162,335` → `get_setting`/`set_setting` (cloud-sync token restore
    silently broken on desktop — catch treats it as "IPC not available yet")
  - `useGatewayStatus.ts:23` → `get_setting`('stripe.api_key') (gateway always shows
    "not configured" on desktop)
- **F-005 (P1, S1)**: Live purchasing screens call desktop-unregistered commands:
  `PurchaseOrdersScreen.tsx:64` → `update_po_status`, `WarehouseConsole.tsx:345` →
  `receive_purchase_order_with_lines` (both registered on tablet; scoped variants are
  registered on desktop). Desktop PO status/receiving flows likely fail at runtime —
  verify on a real desktop build.
- **F-006 (P2, S1)**: Dead IPC surface: 196 desktop / 40 tablet command fns defined but
  never registered (deprecated ADR #7 unscoped family); ~80 dead exports in `ui/src/api`
  (sales/terminals/tables/settings/promotions/purchasing/products/customers). Cleanup +
  drift hazard.
- **F-007 (P2, S1)**: AGENTS violation — 4 production sites bypass `ui/src/api` with
  direct `invoke()`/dynamic import (`UpdateBanner.tsx:211`, `useCloudSync.ts:162,335`,
  `useGatewayStatus.ts:23`). The pre-login justification exists, but the bypass is what
  hid the desktop registration gap.
- **F-008 (P2, S1)**: E2E false confidence: browser-mode E2E answers every command via
  dev-mock (incl. unregistered names), so registration asymmetries cannot surface in CI.
  Recommend a parity gate: every `loggedInvoke`/`invoke` command string must exist in the
  per-shell `generate_handler` list.
- **F-009 (INFO, S1)**: CSP tight on both shells: `script-src 'self'`, `frame-src 'none'`,
  `object-src 'none'`, `upgrade-insecure-requests`; `connect-src` pinned to self +
  github.com + the one cloud host. devCsp only adds localhost Vite ports.
- **F-010 (INFO, S1)**: Desktop `AppState::drop` orderly: aborts plugin hot-reload task,
  signals kernel shutdown. Single `Arc<Mutex<Connection>>` DB model.

---

## S2 — Data Core (oz-core)

**Status:** reviewed (2026-07-25)

**Scope:** `crates/oz-core/` — `db::Store` (hotspot, fan-in 1312), migrations,
Money/i64 representation, schema.

**Key questions**
- All monetary values `i64` minor units via `Money` struct — zero `f32`/`f64` currency leakage.
- Every DB write inside a `rusqlite` transaction; no stray `execute` outside tx.
- Migration hygiene: ordering, idempotency, `fresh_db` test path parity with prod path.
- Store concurrency model (single conn? pool? mutex?) — hotspot fan-in suggests everything touches it.

### S2 Notes

**Checked:** Money representation (canonical impl + all float usage in oz-core),
`Store` design (db/mod.rs + domain files), transaction discipline in write paths,
migration mechanism, unwrap/expect scan, file-size and doc-header rule compliance.

- `Money` canonical in `foundation/src/money.rs`: `minor_units: i64`, checked
  add/sub/mul/div, documented `i64::MIN` edge behavior, safe `format_minor`. `oz-core`
  `money.rs` is a 6-line compatibility re-export.
- Float usage in oz-core (128 matches) confined to non-monetary domains: `popularity.rs`
  (42×, menu-engineering scores) and `table.rs` (pos_x/pos_y/width/height layout coords).
  No currency leakage found.
- `Store<'a>` = thin borrowed-connection wrapper (`&Connection` + optional cache +
  terminal_id); caller owns synchronization and transaction boundaries — explains the
  fan-in 1312 hotspot (everything touches Store; Store itself is stateless).
- Transaction discipline verified in `db/sales.rs`: insert helpers take explicit
  `&Transaction`; `unchecked_transaction` guards concurrent-sale integrity (ADR-19 §5.2);
  `create_sale` (legacy global-db door for oz-cli imports) validates negative qty /
  line_total / tax (MONEY-06/07 audit IDs) before insert. 66 `transaction()` call sites
  across db domain files.
- Migrations: forward-only, compile-time `ALL` array, 17 SQL files
  (`20260813_init.sql` → `20260825_payment_infra.sql`), generic runner; test harness
  unwraps are `// SAFETY:`-commented.
- unwrap/expect scan: 53 occurrences in prod-side files; sampled contexts are
  doc-comments, doctests, and SAFETY-commented test harness code — no prod-path panic
  found in sample.

### S2 Findings

- **F-011 (P2, S2)**: AGENTS file-size rule (prod `.rs` < 1000 lines) violated in
  oz-core: `sales.rs` 2261, `products.rs` 2064, `kds.rs` 1312, `sync_client.rs` 1243,
  `features.rs` 1223, `workspaces.rs` 1106.
- **F-012 (P2, S2)**: 11 oz-core prod files missing the mandatory `//!` module doc
  header: `audit.rs`, `cash_payout.rs`, `config_validator.rs`, `crypto.rs`, `error.rs`,
  `events.rs`, `lib.rs`, `payment.rs`, `rate_limiter.rs`, `session.rs`, `db/tables.rs`.
- **F-013 (INFO, S2)**: Money invariant solid (i64 checked arithmetic, documented
  overflow edges); floats confined to non-monetary domains.
- **F-014 (INFO, S2)**: Transaction discipline strong: explicit tx params, ADR-19 §5.2
  single-tx concurrency guard, MONEY-06/07 validation at the legacy import door.
- **F-015 (INFO, S2)**: Migration system: forward-only, compile-time ordered, 17 files,
  init matches AGENTS' PG baseline name (`20260813_init`).
- **F-016 (P3, S2)**: `features.rs` carries an inline `mod proptests` (line 1056+) in
  addition to its sibling tests file — mild deviation from the sibling-file test rule.

---

## S3 — Platform Layer (core / kernel / startup / sync)

**Status:** reviewed (2026-07-25)

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

**Checked:** rbac.rs + permission_registry.rs design; command-level authorization
coverage via body-level analysis of all desktop scoped commands (session resolution,
permission helper calls, per-command); kernel event_bus; sync conflict strategies;
startup event_handlers; platform file sizes.

- `platform/core/src/rbac.rs`: `<domain>:<action>` permission format, wildcard resolver
  (`*`, `family:*`, `domain:action`), doctested.
- `permission_registry.rs` (ADR #35 D3 / spec 0046): code-resident single source of
  truth; `sensitive` keys (voids, refunds, settlement, role mgmt, bulk export) can never
  ride family wildcards; `validate_grant` rejects unregistered keys.
- Enforcement chain (healthy example): `resolve_session` (token→SessionContext, expiry
  sweep) → `require_*_permission` (DB-backed `require_permission_for_user`) →
  `resolve_store` (scoped connection). The command doc comments correctly state the UI
  role gate is *not* a security boundary.
- Body-level authorization audit: 137 scoped commands check permissions; 169 do not
  (session-scoping only) — see F-017 for the sensitive subset.
- Kernel event bus: in-process topic-based synchronous dispatch, `Send + Sync`, handler
  errors logged-not-propagated (ADR #2).
- Sync: ADR-21 entity-dispatched conflict strategies — Version LWW (product/category/
  tax/user/staff), Sale-status LWW (sale.*/complete/void/refund), CRDT merge (stock.*),
  created-at LWW fallback.
- Startup `event_handlers.rs::fresh_db` (graph hotspot fan-in 268) is a **test helper**
  (in-memory DB + migrations for handler tests) — not a prod path.
- Sizes: platform/sync is the heavyweight — daemon.rs 2061, lib.rs 1850, queue.rs 1490,
  pg_daemon.rs 1331, transport.rs 1167, pg_transport.rs 1022; rbac.rs 1310;
  event_handlers.rs 1053.

### S3 Findings

- **F-017 (P1, S3)**: Systemic RBAC enforcement gap — 169 registered desktop scoped
  commands authenticate + scope but never check permissions. Highlights:
  `rotate_encryption_key_scoped` (ANY valid session rotates encryption keys; `_session`
  unused), 8 gift-card mutations (issue/redeem/top-up/freeze/unfreeze), `adjust_stock_scoped`,
  `create_cash_payout_scoped`, bundle CRUD, supplier/PO lifecycle (6 cmds), store-profile
  CRUD, `pg_sync_start/stop`, workspace recover/suspend; sensitive reads/exports: sales
  history, 24 report commands, EOD/sales-by-hour/daily exports (registry itself declares
  bulk export sensitive), gift-card balances, credit sales. 137 sibling commands do
  enforce — inconsistent adoption, not absent infrastructure. Recommend a command→
  registry-key parity gate.
- **F-018 (P2, S3)**: platform file-size rule violations: 8 files over 1000 lines
  (see S3 Notes sizes), worst `sync/daemon.rs` 2061.
- **F-019 (INFO, S3)**: Permission registry design is strong (sensitivity model,
  wildcard restrictions, validation) — the gap is command-side adoption, not design.
- **F-020 (INFO, S3)**: Event bus design sound; `fresh_db` hotspot is test-only.
- **F-021 (INFO, S3)**: Sync conflict strategy documented and entity-dispatched (ADR-21).

---

## S4 — Business Modules (modules/*)

**Status:** reviewed (2026-07-25)

**Scope:** 14 modules under `modules/`, each with `manifest.json`, `models.rs`,
`repository.rs`, `service.rs`, `error.rs`, sibling `*_tests.rs`.

**Key questions**
- Uniform structure per module (error.rs thiserror, service/repo split, boundary contract tests like `modules/tax/tests/boundary_contract.rs`).
- Manifest ↔ registration parity (kernel manifest test).
- Money handling and tx discipline inside each service.
- `sales` + `tax` + `promotions` interplay — pricing correctness (order of discounts/tax).
- Module fan-in boundaries from graph: `crm`→kernel/oz-payment edges look suspicious, verify.

### S4 Notes

**Checked:** structural uniformity across all 14 modules (manifest/README/Cargo/file
layout), kernel manifest validation, sibling-test presence, tx discipline in module
repositories, pricing/checkout interplay (sales + tax + promotions).

- All 14 modules carry `manifest.json` + `README.md` + `Cargo.toml`; typical layout
  `error/models/repository/service(+commands)`. Sizes small-to-moderate (largest:
  `currency/repository.rs` 996).
- `platform/kernel/tests/module_manifests.rs` validates every `modules/*/manifest.json`
  against the schema (kebab-case id, semver, `dependencies` ↔ `Module::dependencies()`
  parity) — complementary per-module unit tests exist.
- **The module layer is mid-migration**: `modules/promotions/src/lib.rs` is a declared
  stub ("No-op Module implementation. No DB access yet. next: Migrate the promotion rule
  engine and cart-time evaluation into this module") — promotion engine still lives in
  oz-core + desktop commands. `modules/tax` service is a 27-line rate lookup. Several
  modules are thin wrappers over oz-core `db::Store` domains.
- Checkout rigor re-verified in `oz-core/db/sales.rs::complete_sale`: payment split
  validation (`validate_payment_splits_cover_total`) strictly before writes, stock
  deduction batch in-tx, versioned `deduction_locations` JSON; tax-inclusive pricing
  double-charge guard documented (sales.rs:28); tips/service charges as dedicated i64
  columns.
- Only `modules/tax` has a boundary contract test (`tests/boundary_contract.rs`).

### S4 Findings

- **F-022 (P2, S4)**: `modules/currency`: 996-line repository with 16
  INSERT/UPDATE/DELETE statements and **zero transactions** and **zero tests** —
  violates both the tx rule and the sibling-test rule; exchange rates are
  money-adjacent (they feed i64 conversions).
- **F-023 (P2, S4)**: `modules/sales`: 5 prod files, no sibling tests (AGENTS rule).
  Mitigated by heavily-tested `oz-core/db/sales.rs` beneath it.
- **F-024 (INFO, S4)**: Module layer mid-migration to growable-workspace plan;
  promotions stub declared, tax thin — gaps are known and banner-commented.
- **F-025 (INFO, S4)**: Manifest governance solid: kernel integration test covers all
  14 manifests (schema + dependency parity).
- **F-026 (P3, S4)**: Boundary contract tests exist only for tax; pattern not yet
  replicated to sibling modules.

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
| F-004 | 2026-07-25 | S1 | P1 | `UpdateBanner.tsx:211`, `useCloudSync.ts:162,335`, `useGatewayStatus.ts:23` | UI invokes desktop-unregistered `get_setting`/`set_setting`; silent desktop failures (updater persistence, cloud-sync token, gateway status) |
| F-005 | 2026-07-25 | S1 | P1 | `PurchaseOrdersScreen.tsx:64`, `WarehouseConsole.tsx:345` | Live PO flows call `update_po_status` / `receive_purchase_order_with_lines`, unregistered on desktop |
| F-006 | 2026-07-25 | S1 | P2 | desktop/tablet commands + `ui/src/api` | 196/40 dead unregistered command fns; ~80 dead UI api exports — ADR #7 half-migration |
| F-007 | 2026-07-25 | S1 | P2 | `UpdateBanner.tsx`, `useCloudSync.ts`, `useGatewayStatus.ts` | Direct `invoke()` bypasses `ui/src/api` boundary (AGENTS rule) |
| F-008 | 2026-07-25 | S1 | P2 | `ui/e2e`, `ui/src/dev-mock` | Browser-mode E2E mocks all commands; registration asymmetries invisible to CI |
| F-009 | 2026-07-25 | S1 | INFO | `tauri.conf.json` ×2 | CSP tight; connect-src pinned; capabilities default.json (+mobile.json tablet) |
| F-010 | 2026-07-25 | S1 | INFO | `apps/desktop-client/src/state.rs` | Orderly AppState teardown; single-connection SQLite model |
| F-011 | 2026-07-25 | S2 | P2 | `crates/oz-core/src/{sales,products,kds,sync_client,features,workspaces}.rs` | Prod files 1106–2261 lines, over the 1000-line AGENTS limit |
| F-012 | 2026-07-25 | S2 | P2 | 11 files in `crates/oz-core/src` | Missing `//!` module doc headers (AGENTS rule) |
| F-013 | 2026-07-25 | S2 | INFO | `foundation/src/money.rs` | Money: i64 checked arithmetic, documented i64::MIN edges; no currency float leakage |
| F-014 | 2026-07-25 | S2 | INFO | `crates/oz-core/src/db/sales.rs` | Explicit tx params, ADR-19 §5.2 single-tx guard, MONEY-06/07 import validation |
| F-015 | 2026-07-25 | S2 | INFO | `crates/oz-core/migrations/` | Forward-only compile-time migrations, 17 files |
| F-016 | 2026-07-25 | S2 | P3 | `crates/oz-core/src/features.rs:1056` | Inline `mod proptests` alongside sibling tests file |
| F-017 | 2026-07-25 | S3 | P1 | `apps/desktop-client/src/commands/*` (169 cmds) | Scoped commands authenticate but skip permission checks — incl. `rotate_encryption_key_scoped`, gift-card mutations, exports |
| F-018 | 2026-07-25 | S3 | P2 | `platform/sync/src/*`, `platform/core/src/rbac.rs` | 8 platform files over 1000-line limit (worst 2061) |
| F-019 | 2026-07-25 | S3 | INFO | `platform/core/src/permission_registry.rs` | Strong sensitivity model (ADR #35 D3); adoption inconsistent |
| F-020 | 2026-07-25 | S3 | INFO | `platform/kernel/src/event_bus.rs` | Sound synchronous topic bus (ADR #2); fresh_db hotspot is test-only |
| F-021 | 2026-07-25 | S3 | INFO | `platform/sync/src/conflict.rs` | ADR-21 entity-dispatched conflict strategies |
| F-022 | 2026-07-25 | S4 | P2 | `modules/currency/src/repository.rs` | 16 writes, 0 transactions, 0 tests in 996-line repo |
| F-023 | 2026-07-25 | S4 | P2 | `modules/sales/` | No sibling tests in the sales module itself |
| F-024 | 2026-07-25 | S4 | INFO | `modules/promotions/src/lib.rs` | Declared stub — promotion engine still in oz-core; module layer mid-migration |
| F-025 | 2026-07-25 | S4 | INFO | `platform/kernel/tests/module_manifests.rs` | All 14 manifests schema+dependency validated |
| F-026 | 2026-07-25 | S4 | P3 | `modules/tax/tests/` | Boundary contract test pattern not replicated to other modules |

---

## Session log

### 2026-07-25 — Session 1
- Created this journal; mapped repo via codebase-memory graph (architecture overview +
  file tree + crate/app/module inventory).
- Recorded graph hotspots (S0) and pre-existing dirty worktree files (excluded from my commits).
- **S0 reviewed**: workspace members, scale stats (246k Rust LoC / 199k TS LoC), 11 CI
  workflows, methodology declaration, findings F-001..F-003. Committed.
- **S1 reviewed**: programmatic command-registration diff (desktop 567 defined/374
  registered; tablet 403/363), found ADR #7 half-migration with 2 P1s (desktop-silent
  failures F-004, live PO flow breakage F-005), dead IPC surface (F-006), direct-invoke
  violations (F-007), E2E blind spot (F-008). Committed.
- **S2 reviewed**: Money invariant solid (F-013), tx discipline verified (F-014),
  migrations clean (F-015); P2s: 6 oversized files (F-011), 11 missing module headers
  (F-012), inline proptests (F-016). Committed.
- **S3 reviewed**: body-level authorization audit of all desktop scoped commands found
  systemic RBAC gap (169 unguarded cmds incl. encryption-key rotation, gift-card
  mutations, exports — F-017); registry design strong (F-019); event bus + sync
  conflict documented (F-020/F-021); 8 oversized platform files (F-018). Committed.
- **S4 reviewed**: module uniformity + manifest governance solid (F-025); module layer
  mid-migration, promotions is a declared stub (F-024); P2s: currency repo
  no-tx/no-tests (F-022), sales module untested (F-023); boundary-test pattern only in
  tax (F-026). Committed.
- Next: S5 payments/hardware/security crates.
