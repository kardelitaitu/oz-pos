# Validation

## Focused checks

- `cargo fmt --all -- --check`
- `cargo test -p platform-core rbac -- --nocapture`
- `bash scripts/test-tdd.sh -p crates/oz-core`
- `cargo clippy -p platform-core -- -D warnings`
- `bash .agents/skills/skill-drift-guard/scripts/detect.sh`

## Acceptance criteria

- Every permission key enforced anywhere in the codebase is registered
  (bidirectional inventory test: enforced keys == registered keys).
- Sensitive keys (`staff:read_identity`, `staff:read_payroll`,
  `staff:edit_notes`, void/refund/billing/ownership/role-management/export
  keys) can never be granted via a family wildcard.
- Role writes reject unregistered keys and wildcard-flagged-sensitive keys.
- No existing permission string is renamed; the round-172/174 command tests
  stay green unchanged.
- A new operational key in an existing family requires only a registry
  addition — zero role edits.
