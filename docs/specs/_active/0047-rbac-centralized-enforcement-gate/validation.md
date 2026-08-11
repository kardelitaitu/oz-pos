# Validation

## Focused checks

- `cargo fmt --all -- --check`
- `cargo test -p oz-pos-app --lib -- commands::authz -- --nocapture`
- `cargo test -p oz-pos-app --lib -- commands::customers -- --nocapture`
- `cargo test -p oz-pos-tablet --lib -- commands::customers -- --nocapture`
- `cargo clippy -p oz-pos-app -p oz-pos-tablet -- -D warnings`
- `bash .agents/skills/skill-drift-guard/scripts/detect.sh`

## Acceptance criteria

- Every permission-sensitive command passes through the centralized
  `require_permission(permission)` gate; the pinned gated-command set test
  fails for a command that skips it.
- An unregistered permission key or unresolvable role denies by default.
- Round-172 (customers:view) and round-174 (exchange-rate validation) tests
  stay green — the gate migrates the checks, never weakens them.
- Frontend role gating is presentation only; no backend pass depends on it.
