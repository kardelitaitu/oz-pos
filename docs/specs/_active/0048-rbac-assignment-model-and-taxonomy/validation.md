# Validation

## Focused checks

- `cargo fmt --all -- --check`
- `bash scripts/test-tdd.sh -p crates/oz-core`
- `cargo test -p oz-core migrations::tests -- --nocapture`
- `cargo test -p oz-pos-app --lib -- commands::staff -- --nocapture`
- `cargo test -p oz-pos-tablet --lib -- commands::staff -- --nocapture`
- `cargo clippy -p oz-core -p oz-pos-app -p oz-pos-tablet -- -D warnings`
- `cd ui && npx vitest run src/__tests__/StaffManagementScreen.test.tsx src/__tests__/api-staff-contract.test.ts`
- `cd ui && npx tsc --noEmit`

## Acceptance criteria

- Every user has exactly one effective assignment; `users.role_id` rows
  migrate to default global-mode assignments.
- Global-mode roles (Owner, Admin, Auditor) ignore branch and workspace scope.
- Scoped evaluation requires branch and workspace in scope (or explicit `all`);
  empty lists are invalid, never "all".
- `role-cashier` / `role-kitchen` are retired; their users resolve to Staff +
  the workspace scope their current permission set implies.
- Migration round-trip tests pass on a seeded legacy database: behavior
  unchanged, no role references to retired IDs.
- The staff screen presents exactly the five-role taxonomy with no
  cashier/kitchen options, and the assignment editor expresses scope_mode plus
  per-dimension explicit all/list.
- The staff IPC wire shape is pinned by the contract test.
