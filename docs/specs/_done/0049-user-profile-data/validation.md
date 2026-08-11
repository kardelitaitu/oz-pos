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

## Executed results

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | ✅ clean |
| `cargo test -p oz-core --lib` | ✅ 1727/1727 (profile 17/17 incl. encryption-at-rest, mask, read-audit, fail-closed, assign_role_guarded; crypto 3 new; migrations 130 + 131) |
| `cargo test -p oz-core --test staff_integration` | ✅ 25/25 |
| `cargo test -p platform-core --lib` | ✅ 237/237 (incl. profile sensitive-key registry test) |
| `cargo test -p platform-sync --lib` | ✅ 276/276 (incl. snapshot-user residency pin) |
| `cargo test -p oz-pos-app --lib` | ✅ 890/890 (staff 40/40) |
| `cargo test -p oz-pos-tablet --lib` | ✅ 428/428 (staff 19/19) |
| `cargo clippy -p oz-core -p platform-core -p platform-sync -p oz-pos-app -p oz-pos-tablet --lib --tests -- -D warnings` | ✅ clean on the changed area (2 pre-existing errors in `topology.rs`, untouched by 0049) |
| `cd ui && npx vitest run src/__tests__/StaffManagementScreen.test.tsx src/__tests__/api-staff-contract.test.ts` | ✅ 17/17 + 4/4 |
| `cd ui && npx tsc --noEmit` | ✅ clean |
| `cd ui && npm run lint` | ✅ 0 errors (8 pre-existing warnings in other files) |
| `python scripts/verify-bundle-parity.py` | ✅ 0 missing keys (en + id) |
| `bash .agents/skills/skill-drift-guard/scripts/detect.sh` | ✅ no drift |

## Acceptance criteria

- ✅ Creation requires the 9 mandatory fields with field-specific validation
  (national ID per type + UNIQUE when present, email format + UNIQUE, phone
  E.164, DOB not in the future, monthly take-home pay > 0 minor units) —
  `UserProfile::validate` matrix test + `create_user_with_profile` rejects
  incomplete; uniqueness via email index + national-id hash index.
- ✅ Legacy rows enter the incomplete-profile state: checkout login works
  (`legacy_create_user_leaves_incomplete_profile`), user flagged in staff
  management (`is_profile_complete` DTO + badge test), management-role
  assignment and sensitive grants gated (`assign_role_guarded` /
  `require_role_assignable` test; UI disables role + workspace controls).
- ⚠️ `national_id` and `monthly_take_home_minor` encrypt at rest and decrypt
  on explicit grant; failures fail closed — implemented in `oz_core::crypto`
  (domain-separated AES-GCM) rather than the oz-security keyring because
  oz-security depends on oz-core (cycle is impossible); deviation recorded in
  JOURNAL.md round 179.
- ✅ `national_id` displays as last-4 by default; full value only via the
  explicit grant (`get_user_profile_viewed_by` + `mask_last4` test; UI
  renders the masked value only); audit events record access, never values
  (`view_with_grants_returns_full_values_and_audits`).
- ✅ Sensitive fields never appear in cloud sync or bulk export payloads —
  `SnapshotUser` wire-format pin test; the sync upsert touches no profile
  columns.
- ✅ Deactivation never deletes identity, payroll, or emergency contact data
  (`deactivation_preserves_profile`).
- ✅ The staff form enforces the 9 required fields with localized per-field
  errors (`validateProfileForm` + inline field-error test); national_id
  renders last-4 in list (`renders the national id masked to last-4` test);
  incomplete-profile users are flagged with management controls disabled
  (`flags incomplete-profile users` + `disables role and workspace
  assignment` tests).
- ✅ The staff IPC wire shape including the profile fields is pinned by
  `api-staff-contract.test.ts` (4/4).
