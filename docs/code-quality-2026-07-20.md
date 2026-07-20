# Code Quality Audit — 0.0.14

## P36-1: Dead Code

`cargo doc` compiled successfully. 27 `#[allow(dead_code)]` annotations found — all intentional:

| File | Count | Rationale |
|------|-------|-----------|
| `oz-hal/drivers/escpos.rs` | 7 | ESC/POS command enums (barcode types, print modes) — used by trait, not directly |
| `oz-payment/drivers/*` | 6 | Gateway error variants + test fixture fields |
| `cloud-server/webhooks.rs` | 3 | Webhook event type discriminators |
| `cloud-server/db.rs` | 3 | DB pool config fields |
| `foundation/contracts.rs` | 1 | Contract status enum variant reserved for future |
| `oz-lua/lib.rs` | 1 | Sandbox internals accessible via FFI |
| `oz-core/cache.rs` | 1 | Cache eviction policy field |
| `platform/startup/rate_sync.rs` | 1 | Currency rate sync config |
| `desktop-client/workspaces.rs` | 1 | Workspace registry lookup |
| `oz-hal/serial_display.rs` | 1 | Display command enum |

**Verdict:** No dead code to remove. All 27 are intentional suppressions with documented rationale.

## P36-2: `cargo doc` Coverage

`cargo doc --workspace --no-deps` generated successfully. Warnings found in 2 crates:

- **`foundation`**: 2 warnings — `Sku` Display impl, `CartId` new()
- **`oz-core`**: 20 warnings — mostly database module internal functions

**Recommendation:** Add `///` doc comments to the 22 flagged public items. Non-blocking — all critical public API is already documented.

## P36-3: TODO/FIXME/HACK Audit

5 items found across the codebase:

| File | Text | Status |
|------|------|--------|
| `sync_api.rs:293` | TODO: add POST endpoints for tax_rates and users | Deferred feature |
| `location_resolver.rs:464` | TODO(ADR-19): greedy-fill across locations | ADR-19 in progress |
| `db/workspaces.rs:354` | TODO(ADR #4): user_store_access check | ADR-4 deferred |
| `db/workspaces.rs:1198` | TODO(ADR #5): public archive_instance() | ADR-5 deferred |
| `currency_integration.rs:463` | `"XXX"` in test (not a real TODO) | Test-only — intentionally invalid |

**Verdict:** All 5 are deferred features or test-only artifacts. No immediate action needed.
