# Validation — 0048 assignment model and role taxonomy

**Status: needs-human-approval — 2026-08-11.** All cycles executed (1, 2a,
2b, 2c, 3 — see plan §10). Criteria below are marked ✅ (met). The sole
remaining step is the maintainers' approval to move the spec to `_done`.

## Executed checks (all cycles)

| Check | Command | Result |
|---|---|---|
| oz-core lib (full) | `cargo test -p oz-core --lib` | ✅ 1746/1746 (assignments 13/13 incl. `set_assignment` write + rollback join, profile 17/17, migration_128/129 2/2) |
| Migration registry | `cargo test -p oz-core --lib -- migrations::tests` | ✅ incl. `migration_128_backfills_assignments_from_legacy_role_ids`, `migration_129_retires_cashier_kitchen` |
| Staff integration | `cargo test -p oz-core --test staff_integration` | ✅ green |
| platform-core | `cargo test -p platform-core --lib` | ✅ 236/236 (retirement regression: no preset id is cashier/kitchen) |
| Desktop app | `cargo test -p oz-pos-app --lib` | ✅ 893/893 (staff 41/41 incl. `scoped_update_staff_writes_assignment_scope_atomically`; authz 26/26) |
| Tablet app | `cargo test -p oz-pos-tablet --lib` | ✅ 429/429 (staff 19/19; authz green) |
| gate_audit census | `cargo test -p oz-pos-app --test gate_audit` | ✅ 3/3 (staff.rs pin bumped 5→6 for 0049's `get_staff_profile_scoped`) |
| Formatting | `cargo fmt --all -- --check` | ✅ clean |
| Lint | `cargo clippy -p oz-core -p oz-pos-app -p oz-pos-tablet --lib --tests -- -D warnings` | ✅ clean (changed area) |
| Drift guard | `bash .agents/skills/skill-drift-guard/scripts/detect.sh` | ✅ no drift |
| Bundle parity | `python scripts/verify-bundle-parity.py` | ✅ 0 missing keys (new assignment keys in both `staff.ftl` + `staff.id.ftl`) |
| i18n lint + FTL dedupe | `bash scripts/lint-i18n.sh` / `python scripts/dedupe-ftl.py --dry-run` | ✅ clean |
| UI typecheck | `cd ui && npx tsc --noEmit` | ✅ staff screen + contract clean (only pre-existing foreign retail WIP errors remain) |
| UI lint | `cd ui && npm run lint` | ✅ staff screen clean (only pre-existing foreign retail WIP errors remain) |
| Staff screen tests | `cd ui && npx vitest run src/__tests__/StaffManagementScreen.test.tsx` | ✅ 21/21 (taxonomy dropdown, editor pre-fill, scoped save, empty-list block) |
| IPC contract test | `cd ui && npx vitest run src/__tests__/api-staff-contract.test.ts` | ✅ 7/7 (assignment wire shape pinned) |

## Acceptance criteria

- ✅ **Every user has exactly one effective assignment; `users.role_id`
  rows migrate to default global-mode assignments.** `assignments.user_id`
  is the primary key; migration 128 backfills every legacy row
  (round-trip test), and `create_user` / `create_user_with_profile` write
  one on user creation.
- ✅ **Global-mode roles (Owner, Admin, Auditor) ignore branch and workspace
  scope.** `matches_scope` for `Global` ignores dimensions; pinned by
  `matches_scope_global_ignores_dimensions` and
  `gate_scoped_global_assignment_ignores_scope`.
- ✅ **Scoped evaluation requires branch and workspace in scope (or
  explicit `all`); empty lists are invalid, never "all".** Pinned by the
  `matches_scope_*` matrix (all/one/combination, empty-list-denies,
  `None`-context-denies) and `gate_scoped_denies_*` at the gate; the
  write path (`set_assignment`) replaces dimension rows so stale grants
  never survive a scope change, and the staff screen blocks saving a
  scoped assignment with an empty list dimension.
- ✅ **`role-cashier` / `role-kitchen` are retired; their users resolve to
  Staff + the workspace scope their current permission set implies.**
  Migration 129 re-points `users.role_id` / `assignments.role_id` to
  `role-staff` and deletes the role rows; CASHIER/KITCHEN presets +
  constants are gone; the ~22-file seed sweep maps staff-like fixtures to
  `role-staff` and limited-access assertions to a narrow custom role; the
  retirement regression test pins no preset id is cashier/kitchen.
- ✅ **Migration round-trip tests pass on a seeded legacy database:
  behavior unchanged, no role references to retired IDs.** Round-trip +
  idempotency pass for 128/129; the final reference census shows only
  intentional remaining mentions (a historical comment and the
  retirement regression test itself).
- ✅ **The staff screen presents exactly the five-role taxonomy with no
  cashier/kitchen options, and the assignment editor expresses scope_mode
  plus per-dimension explicit all/list.** The dropdown filters to the five
  preset ids (Owner → Admin → Manager → Staff → Auditor); the editor has a
  global | scoped radio and per-dimension branch (store profiles) +
  workspace pickers with explicit all/list toggles; the workspace column
  derives from the DTO assignment.
- ✅ **The staff IPC wire shape is pinned by the contract test.** The DTO
  carries `assignment` (scope_mode, branches_all, branch_ids,
  workspaces_all, workspace_keys); create/update args carry the optional
  assignment and the backend writes it atomically with the user +
  profile (in-tx writer, no nested BEGIN).
