# Validation

## Focused checks

- `cargo fmt --all -- --check`
- `bash scripts/test-tdd.sh -p crates/oz-core`
- `cargo test -p oz-core migrations::tests -- --nocapture`
- `cargo test -p oz-pos-app --lib -- commands::staff -- --nocapture`
- `cargo test -p oz-pos-tablet --lib -- commands::staff -- --nocapture`
- `cargo test -p oz-security -- --nocapture`
- `cargo clippy -p oz-core -p oz-pos-app -p oz-pos-tablet -p oz-security -- -D warnings`
- `cd ui && npx vitest run src/__tests__/StaffManagementScreen.test.tsx src/__tests__/api-staff-contract.test.ts`
- `cd ui && npx tsc --noEmit`
- `bash .agents/skills/skill-drift-guard/scripts/detect.sh`

## Acceptance criteria

- Creation requires the 9 mandatory fields with field-specific validation
  (national ID per type + UNIQUE when present, email format + UNIQUE, phone
  E.164, DOB not in the future, monthly take-home pay > 0 minor units).
- Legacy rows enter the incomplete-profile state: checkout login works, user
  flagged in staff management, management-role assignment and sensitive grants
  gated.
- `national_id` and `monthly_take_home_minor` encrypt at rest (oz-security
  keyring) and decrypt on explicit grant; failures fail closed.
- `national_id` displays as last-4 by default; full value only via the
  explicit grant; audit events record access, never values.
- Sensitive fields never appear in cloud sync or bulk export payloads.
- Deactivation never deletes identity, payroll, or emergency contact data.
- The staff form enforces the 9 required fields with localized per-field
  errors; national_id renders last-4 in list and detail; incomplete-profile
  users are flagged with management controls disabled.
- The staff IPC wire shape including the profile fields is pinned by the
  contract test.
