# RBAC assignment model and role taxonomy alignment

## 1. Decision requested

Introduce the assignment model ADR #35 D5 specifies (`assignments` with
`scope_mode` + branch/workspace scopes), migrate `users.role_id` to a default
global-mode assignment, and align the role taxonomy to D4 (seed Owner, Admin,
Auditor; fold cashier/kitchen into Staff + workspace assignments; retire
`role-cashier`/`role-kitchen`). This is D9 steps 3–4 and depends on 0046/0047.

## 2. Evidence baseline

- `modules/staff/src/models.rs` defines the six today roles: `role-owner`,
  `role-manager`, `role-cashier`, `role-kitchen`, `role-staff`, `role-custom`.
- `users.role_id` is a single global column (migration `007_customers.sql`);
  tests seed `('user-cashier', ..., 'role-cashier')` rows throughout
  `apps/*/src/commands/*.rs`.
- `role-cashier` grants include `sales:process` (+ payment/customer/discount
  keys); `role-kitchen` grants include `kds:view`, `kds:update`, `sales:view`,
  `workspaces:switch` (`crates/oz-core/src/db/staff.rs` tests assert `kds:view`
  / `kds:update`).
- Branch and workspace entities exist (`stores`, `terminals`,
  `workspaces`); the audit's CUR-03 flagged unscoped currency commands as a P0
  that this model is the structural fix for.
- Migration conventions: numbered `.sql` files registered in
  `crates/oz-core/src/migrations.rs`; data migrations repair existing rows
  (see `117_scoping_store_id_fk.sql`).

## 3. Problem statement

A global `role_id` cannot express "Manager for Branch A and Branch B, limited
to warehouse and retail-pos" or "Staff for the kds workspace" — the exact
shapes ADR #35 makes the product's core. And the six-role set contradicts D4's
five-role taxonomy, with cashier/kitchen as roles instead of Staff +
workspace assignments. The slice replaces the flat model without breaking
existing databases.

## 4. Scope of the slice

### 4.1 Schema

New migration: `assignments` (`id`, `user_id`, `role_id`, `scope_mode`
`global`|`scoped`, `expires_at` deferred), `assignment_branches`
(`assignment_id`, `branch_id`), `assignment_workspaces` (`assignment_id`,
`workspace_key`). Unique constraint: one effective assignment per user.

### 4.2 Migration behavior

- `users.role_id` rows become a default **global-mode** assignment — behavior
  unchanged for existing databases.
- Cashier/kitchen users map to Staff with the workspace scope their current
  permission set implies (`retail-pos` / `kds`); `role-cashier` and
  `role-kitchen` rows are retired after remapping.
- Owner keeps global mode; Manager keeps global-mode default (scoped mode is a
  new capability, not a forced change).

### 4.3 Evaluation

Scoped mode: branch dimension and workspace dimension, each explicit `all` or
a list. Global mode: both dimensions ignored. Request authorization requires
permission (gate) AND branch/workspace in scope for scoped roles.

### 4.4 Front-end (ui/src)

- The staff screen's role list renders the five-role taxonomy; cashier/kitchen
  are never selectable roles — their labels derive from workspace assignment
  and the optional `job_title` field.
- The existing assignment editor (StaffManagementScreen already manages
  workspace assignments) gains scope_mode and the branch dimension; each
  dimension is an explicit all or a list.
- New strings land in both `staff.ftl` bundles (en + id) to pass the parity
  gate; the staff IPC wire shape (`RoleDto`, staff args) is pinned by a new
  `api-staff-contract.test.ts` — no staff contract test exists today.

## 5. Implementation plan

1. Write migration tests first (Red): default-assignment round-trip,
   role-retirement mapping, scope evaluation (all/one/combination),
   global-ignores-scope.
2. Add the migration + runner registration (Green).
3. Add the assignment evaluation API and migrate role resolution in the gate
   (0047) to consult assignments.
4. Seed Owner/Admin/Auditor; keep `role-manager`/`role-staff`; retire
   cashier/kitchen via the migration.
5. Update test seeds that hardcode `role-cashier`/`role-kitchen` to the new
   Staff + workspace shape.
6. Update the staff screen: five-role list, assignment editor (scope_mode +
   branch/workspace pickers), localized strings, and the new staff IPC
   contract test.
7. Run area tests: `test-tdd.sh -p crates/oz-core`, `cargo test -p oz-pos-app
   --lib`, `cargo test -p oz-pos-tablet --lib`, fmt, clippy, plus the UI
   checks from validation.md.

## 6. Test plan

### Existing tests to modify (the role-ID sweep)

Seeds hardcode `('user-cashier', ..., 'role-cashier')` and
`('user-kitchen', ..., 'role-kitchen')` across command tests; re-point them
to the Staff + workspace-assignment shape, keeping behavior assertions
unchanged:

- Desktop commands: `auth.rs`, `authz.rs`, `categories.rs`, `customers.rs`,
  `inventory.rs`, `loyalty.rs`, `shifts.rs`, `staff.rs`, `stock_transfers.rs`,
  `tax.rs`, `topology.rs`, `workspaces.rs`.
- Tablet commands: `auth.rs`, `authz.rs`, `categories.rs`, `customers.rs`,
  `loyalty.rs`, `pos.rs`, `settings.rs`, `tax.rs`, `workspaces.rs`.
- Integration/other: `crates/oz-core/tests/staff_integration.rs`,
  `settings_integration.rs`, `shift_integration.rs`,
  `crates/oz-cli/src/commands.rs`.
- Role sources: `modules/staff/src/models.rs` (consts),
  `modules/staff/src/lib.rs` (`Role::new("role-cashier")`),
  `apps/cloud-server/src/openapi.rs` (docs/examples),
  `apps/desktop-client/src/state.rs` (session resolution).

### New tests (Red first)

- Migration round-trip: a legacy DB with `role-cashier`/`role-kitchen` rows
  upgrades to default global-mode assignments; retired IDs are unreferenced.
- Role-retirement mapping: cashier → Staff + `retail-pos`, kitchen → Staff +
  `kds` (the workspace scope their current grants imply).
- Scope evaluation matrix: `all`/one/combination per dimension; denial when
  branch or workspace is out of scope.
- Global-ignores-scope: Owner/Admin/Auditor assignments ignore both dimensions.
- One-effective-assignment invariant per user.

### UI tests (new)

- `StaffManagementScreen.test.tsx` — the role list shows exactly the five
  roles; cashier/kitchen are absent; the assignment editor drives scope_mode
  and the per-dimension all/list pickers.
- `api-staff-contract.test.ts` (new) — pins the RoleDto / assignment wire
  shape consumed by the screen.

## 7. Security and correctness considerations

- Fail closed: an assignment with an unknown role or scope_mode never grants.
- "All" is explicit — empty lists are invalid, never "all" (ADR #35 D5).
- The migration is atomic and re-runnable; existing DBs are behavior-unchanged,
  which the round-trip tests pin.

## 8. Non-goals

- Org-tenant layer, invitations, expiry, caching (ADR #35 D7).
- Custom-role UI (D9 step 5).
- Profile fields (0049).
- Role inheritance (permanently out of scope).

## 9. Rollback plan

The migration is additive (new tables) plus a data remap of role IDs. Rollback
restores the old seeds and keeps the new tables unused. If the role retirement
breaks an edge case, the remap can be scoped down to a later step without
reverting the assignment tables.
