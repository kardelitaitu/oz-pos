# Validation — 0048 assignment model and role taxonomy

**Status: IN PROGRESS — 2026-08-11.** Cycles 1, 2a, 2b executed (see plan
§10); cycles 2c (retirement + seed sweep) and 3 (UI) pending. Criteria
below are marked ✅ (met), ⏳ (open), or 🚧 (partial).

## Executed checks (cycles 1 + 2a + 2b)

| Check | Command | Result |
|---|---|---|
| oz-core lib (full) | `cargo test -p oz-core --lib` | ✅ 1705/1705 (assignments 12/12, migration_128 1/1, gate/write 8 new) |
| Migration registry | `cargo test -p oz-core --lib -- migrations::tests` | ✅ incl. `migration_128_backfills_assignments_from_legacy_role_ids`, `expected_tables` |
| Staff integration | `cargo test -p oz-core --test staff_integration` | ✅ 25/25 |
| platform-core presets | `cargo test -p platform-core --lib` | ✅ 236/236 (admin/auditor preset tests) |
| Desktop staff/authz | `cargo test -p oz-pos-app --lib -- commands::authz commands::staff` | ✅ 46/46 |
| Tablet staff/authz | `cargo test -p oz-pos-tablet --lib -- commands::authz commands::staff` | ✅ 24/24 |
| Formatting | `cargo fmt --all -- --check` | ✅ clean |
| Lint | `cargo clippy -p platform-core -p oz-core -p oz-pos-app -p oz-pos-tablet -- -D warnings` | ✅ clean |
| Drift guard | `bash .agents/skills/skill-drift-guard/scripts/detect.sh` | ✅ no drift |
| UI checks | `cd ui && npx vitest run ...StaffManagementScreen... ...api-staff-contract...` + `tsc --noEmit` | ⏳ cycle 3 |

## Acceptance criteria

- ✅ **Every user has exactly one effective assignment; `users.role_id`
  rows migrate to default global-mode assignments.** `assignments.user_id`
  is the primary key; migration 128 backfills every legacy row
  (round-trip test), and `create_user` writes one on user creation.
- ✅ **Global-mode roles (Owner, Admin, Auditor) ignore branch and workspace
  scope.** `matches_scope` for `Global` ignores dimensions; pinned by
  `matches_scope_global_ignores_dimensions` and
  `gate_scoped_global_assignment_ignores_scope`.
- ✅ **Scoped evaluation requires branch and workspace in scope (or
  explicit `all`); empty lists are invalid, never "all".** Pinned by the
  `matches_scope_*` matrix (all/one/combination, empty-list-denies,
  `None`-context-denies) and `gate_scoped_denies_*` at the gate.
- ⏳ **`role-cashier` / `role-kitchen` are retired; their users resolve to
  Staff + the workspace scope their current permission set implies.**
  Cycle 1 backfills the users (staff + retail-pos/kds); the retirement of
  the role rows/presets and the seed sweep are cycle 2c.
- 🚧 **Migration round-trip tests pass on a seeded legacy database:
  behavior unchanged, no role references to retired IDs.** Round-trip +
  idempotency pass; "no references to retired IDs" is pending 2c.
- ⏳ **The staff screen presents exactly the five-role taxonomy with no
  cashier/kitchen options, and the assignment editor expresses scope_mode
  plus per-dimension explicit all/list.** Cycle 3 (UI).
- ⏳ **The staff IPC wire shape is pinned by the contract test.** Cycle 3
  (`api-staff-contract.test.ts`).
