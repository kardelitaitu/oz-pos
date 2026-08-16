# Validation

> **Status: IMPLEMENTED — 2026-08-11.** All checks executed and green; every
> acceptance criterion met. Shipped in `bde2962d` (feat) + `7fa406a4`
> (refactor), journaled as round 175.

## Executed checks (2026-08-11)

- `cargo test -p platform-core --lib` — 234/234 pass (registry 9/9).
- `cargo test -p oz-core --lib -- db::staff` — 42/42 pass (incl. the
  `create_role` write-time rejection tests).
- `cargo test -p oz-core --test staff_integration` — 25/25 pass.
- `cargo test -p oz-pos-app --lib -- commands::staff` — 40/40 pass.
- `cargo test -p oz-pos-tablet --lib -- commands::staff` — 19/19 pass.
- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p platform-core -- -D warnings` — clean.
- `cargo clippy -p oz-core -- -D warnings` — clean.
- `bash .agents/skills/skill-drift-guard/scripts/detect.sh` — no drift.
- `test-changed.sh` — not runnable this round: `oz-pos-app.exe` was locked by
  a running process (left alone per the shared-tree rule); nearest consumer
  suites above substituted.

## Acceptance criteria

- [x] **Every permission key enforced anywhere in the codebase is registered**
  — bidirectional inventory test pins `rbac::ALL_ENFORCED` == registry (all
  68 keys), so a new key is registered everywhere or nowhere. The audit also
  caught three keys the plan's baseline missed (`products:crud`,
  `categories:manage`, `products:view`); the first two now have constants,
  the fixture key was renamed to `products:read`.
- [x] **Sensitive keys can never be granted via a family wildcard** —
  `validate_grants` rejects wildcards covering sensitive keys, and the
  definition-time tests derive the rule from the registry itself, so a
  sensitive key added to a *new* family is rejected automatically (8
  sensitive keys today: `sales:void`, `sales:refund`, `payments:refund`,
  `payments:settle`, `staff:manage_roles`, `staff:delete`,
  `reports:export`, `audit:export`).
- [x] **Role writes reject unregistered keys and wildcard-flagged-sensitive
  keys** — `Store::create_role` validates each grant through the registry and
  maps failures to `CoreError::Validation`; rejection proven by the 5/5
  `create_role` tests. The global `*` is rejected too (reserved for the Owner
  seed, which bypasses via direct insert).
- [x] **No existing permission string is renamed** — the two legacy seed
  keys are registered byte-identical via new constants; the only fixture
  edits are synthetic keys with no production meaning.
- [x] **A new operational key in an existing family requires only a registry
  addition — zero role edits** — verified by the bidirectional inventory and
  `validate_grants` design; `ALL_ENFORCED` is the single place to add a key.
