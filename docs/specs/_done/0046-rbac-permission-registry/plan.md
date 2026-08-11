# RBAC code-resident permission registry

> **Status: IMPLEMENTED — 2026-08-11.** Shipped in two commits
> (`bde2962d`, `7fa406a4`); moved to `_done/`. See §10 for the completion
> record. The sections below are the original plan as approved.

## 1. Decision requested

Build the code-resident permission registry that ADR #35 D3 requires: every
permission key the codebase enforces, classified into a family with a
sensitive flag and a description, validated at role-write time so unregistered
keys and wildcarded sensitive keys are rejected. This is D9 step 1 — the
foundation every later slice builds on.

## 2. Evidence baseline

Verified 2026-08-11:

- `platform/core/src/rbac.rs` documents and implements wildcard matching —
  `has_permission(&["sales:*"], "sales:process")` and `"*"` — with permission
  consts such as `SALES_VOID`, `LOYALTY_MANAGE`, `CUSTOMERS_VIEW`.
- Role permissions are stored as JSON arrays in `roles.permissions`
  (migration `007_customers.sql`; seeds like
  `'["sales:process","sales:void","products:crud"]'`).
- Module manifests declare per-module permission lists:
  `modules/sales/manifest.json` (`sales:void`, `sales:refund`, `reports:view`),
  consumed by `platform/kernel/src/manifest.rs`.
- Enforced strings observed in seeds/tests/commands: `sales:process`,
  `sales:void`, `sales:refund`, `sales:override_price`, `sales:view`,
  `products:crud`, `products:view`, `categories:manage`, `inventory:adjust`,
  `reports:view`, `customers:view`, `customers:create`, `kds:view`,
  `kds:update`, `shifts:view_any`, `staff:manage_roles`, `*` (owner).

## 3. Problem statement

Nothing today classifies a key as operational (wildcard-eligible) or sensitive
(explicit-only), so ADR #35's stability rule ("a new sensitive action must be
granted explicitly, never via a family wildcard") is unenforceable. Role
writes accept any string, so a typo or a sensitive key under a wildcard passes
silently. The registry makes classification a tested, reviewable artifact and
gives every later slice (gate, assignments, profile) a single source of truth.

## 4. Scope of the slice

### 4.1 Registry shape

A code module (adjacent to `rbac.rs`) exposing, for every key: the key string,
its family, a `sensitive: bool`, and a description. The inventory of currently
enforced keys is pinned by a test that fails if any enforced key is missing
from the registry or any registry key is not enforced (bidirectional).

### 4.2 Classification rule

Sensitive per ADR #35 D2: voids, refunds, billing, ownership, role
management, bulk export, and staff identity/payroll/notes keys
(`staff:read_identity`, `staff:read_payroll`, `staff:edit_notes`). Everything
else is operational. Owner's `"*"` grant is the single documented exception
and is represented explicitly, not as a template.

### 4.3 Write-time validation

Role grant writes validate each key against the registry: unregistered keys
are rejected; a sensitive key requested via a family wildcard is rejected.
The registry rejects any wildcard covering a sensitive key at definition time
(compile-time assertion where practical).

## 5. Implementation plan

1. Add the registry module with the pinned key inventory and classification.
2. Add the bidirectional inventory test (enforced keys == registered keys).
3. Add wildcard-vs-sensitive and unknown-key rejection tests (Red).
4. Wire write-time validation into the role write path (Green).
5. Update module manifests/consts only where they must reference the registry;
   existing strings stay byte-identical.
6. Run area tests: `cargo test -p platform-core`, `test-tdd.sh -p crates/oz-core`,
   `cargo fmt --all -- --check`, `cargo clippy -p platform-core -- -D warnings`.

## 6. Test plan

### Existing tests to extend (none break — strings stay byte-identical)

- `platform/core/src/rbac.rs` — existing wildcard-matching tests stay;
  extend with registry lookups.
- `crates/oz-core/tests/staff_integration.rs` —
  `role_permissions_json_roundtrip` and the seed assertions stay; extend with
  registry validation on the same seeds.

### New tests (Red first)

- Bidirectional inventory: every enforced key is registered and every
  registered key is enforced (fails until the classification is complete).
- A family wildcard covering a sensitive key is rejected at definition time.
- Role writes reject unregistered keys.
- Role writes reject a sensitive key granted via a family wildcard.

## 7. Security and correctness considerations

- The registry is code, never a database table (ADR #35 D3).
- Deny-by-default: an unknown key anywhere fails loudly (write rejection or
  test failure), never silently passes.
- No existing permission string is renamed — renames churn every enforcement
  call site for zero user value.

## 8. Non-goals

- The centralized gate (0047), assignment model (0048), profile fields (0049).
- A runtime-editable registry.
- Renaming or restructuring existing strings.

## 9. Rollback plan

The registry is additive and non-runtime-breaking: removing it reverts role
writes to today's unchecked behavior. Each validation rule ships behind its
own test, so a rule that proves too strict can be removed individually
without reverting the registry itself.

## 10. Completion record (2026-08-11)

**Shipped:** `platform-core::permission_registry` — all 68 enforced keys
classified by family + `sensitive: bool`, a bidirectional inventory test
(constants == registry), and `validate_grants` wired into
`Store::create_role` → `CoreError::Validation`. Rejects unregistered keys,
wildcards covering sensitive keys, and the global `*` (reserved for the Owner
seed, which bypasses via direct insert). Added `PRODUCTS_CRUD` /
`CATEGORIES_MANAGE` constants for the legacy seed keys (byte-identical
strings).

**Deviations from plan (all recorded in JOURNAL round 175):**

- The audit surfaced keys the plan's baseline missed: legacy seeds use
  `products:crud` and `categories:manage` (no constants existed), and a test
  fixture used `products:view` (nothing enforces it) — fixed by adding the two
  constants and renaming the fixture key to `products:read`.
- Two integration fixtures used synthetic keys (`module:N:action`, `["test"]`)
  and were updated to registered keys.
- Finalization (commit `7fa406a4`) consolidated the four copies of the key
  inventory into `rbac::ALL_ENFORCED` and derived the wildcard-vs-sensitive
  tests from the registry, so a sensitive key added to a new family is pinned
  automatically (−122 lines).

**Verify (round 175 + finalization):** registry 9/9, oz-core lib 1678/1678,
staff_integration 25/25, oz-pos-app staff 40/40, oz-pos-tablet staff 19/19,
platform-core 234/234 post-refactor; `cargo fmt --check`, clippy
`-D warnings` (platform-core + oz-core), and drift guard all clean.
`test-changed.sh` blocked by the locked `oz-pos-app.exe` (running process,
left alone).

**Commits:** `bde2962d` (feat) + `7fa406a4` (refactor). Journal round 175;
CHANGELOG bullet under Added.

**Known follow-ups (deferred, not regressions):** module-manifest
`permissions` arrays are a separate declarative DSL (format-validated only),
not RBAC enforcement — a future slice may reconcile them. The centralized
enforcement gate (0047) consumes this registry's public API
(`is_registered`, `is_sensitive`, `validate_grant`).
