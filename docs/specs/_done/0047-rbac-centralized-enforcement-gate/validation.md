# Validation — 0047 centralized fail-closed enforcement gate

**Status: IMPLEMENTED — 2026-08-11.** All focused checks executed; all
acceptance criteria met. See plan §10 for the completion record.

## Executed checks

| Check | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | ✅ clean |
| Gate behavior (oz-core) | `cargo test -p oz-core --lib -- db::staff` | ✅ 50/50 (gate 8/8) |
| Desktop migration contract | `cargo test -p oz-pos-app --lib -- commands::authz commands::customers commands::exchange_rates` | ✅ 56/56 |
| Tablet migration contract | `cargo test -p oz-pos-tablet --lib -- commands::authz commands::customers commands::exchange_rates` | ✅ 55/55 |
| Pinned gated-command census | `cargo test -p oz-pos-app --test gate_audit` | ✅ 3/3 |
| Lint | `cargo clippy -p oz-core -- -D warnings` | ✅ clean |
| Lint | `cargo clippy -p oz-pos-app --lib -- -D warnings` | ✅ clean |
| Lint | `cargo clippy -p oz-pos-tablet --lib -- -D warnings` | ✅ clean |
| Drift guard | `bash .agents/skills/skill-drift-guard/scripts/detect.sh` | ✅ no drift |

Note: `test-changed.sh` could not run — the app binaries are held open by
running processes (`oz-pos-app` via another agent's `cargo run`,
`oz-pos-tablet` via `tauri dev`) and were left alone per the shared-tree
rule. The area-scoped suites above and a direct execution of the built
`gate_audit` harness against current sources cover the changed area.

## Acceptance criteria

- ✅ **Every permission-sensitive command passes through the centralized
  `require_permission(permission)` gate; the pinned gated-command set test
  fails for a command that skips it.** `gate_audit.rs` pins the full census
  of both clients bidirectionally; its Red run (deliberately corrupted pin)
  failed with `kds.rs gate-call count drifted: pin says 14, source has 15`,
  proving drift detection, then went green 3/3 with the true pin. An
  enforcement sweep found zero `.authorize()`/`has_permission()` callers in
  `apps/` or `modules/` outside the gate.
- ✅ **An unregistered permission key or unresolvable role denies by
  default.** `gate_denies_unregistered_permission_even_for_owner` (typo key
  denies the `*` Owner grant), `gate_denies_unknown_user`,
  `gate_denies_inactive_user`, and `gate_denies_user_with_unresolvable_role`
  (FKs off, role row deleted) all assert `CoreError::PermissionDenied`.
- ✅ **Round-172 (customers:view) and round-174 (exchange-rate validation)
  tests stay green — the gate migrates the checks, never weakens them.**
  Desktop `customers`/`exchange_rates` suites 56/56, tablet 55/55 — including
  the round-172/174 denial and validation tests, unmodified.
- ✅ **Frontend role gating is presentation only; no backend pass depends on
  it.** The gate resolves the caller's role from the database
  (`Store::require_permission`), never from frontend-supplied input; the
  client wrappers map only the denial error to the existing `permissionDenied`
  wire shape.
