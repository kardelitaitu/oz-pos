# User profile data contract with sensitive-field handling

## 1. Decision requested

Implement ADR #35 D6 end to end: the 9 mandatory-at-creation profile fields
plus the optional set, the incomplete-profile state, the sensitive-field
handling (explicit grants via the 0046 registry, at-rest encryption, read
audit, last-4 masking, cloud-sync/export residency, compliance retention).
This is D9 step 6 and depends on 0046–0048.

## 2. Evidence baseline

- `users` today: `id`, `username UNIQUE`, `pin_hash` (argon2), `display_name`,
  `role_id`, `is_active`, timestamps (migration `007_customers.sql`).
- Money convention: i64 minor units (`total_spent_minor`, `opening_balance_minor`);
  `customers.notes` is `TEXT NOT NULL DEFAULT ''`.
- `oz-security` provides keyring-backed encryption (license-API-key precedent:
  `apps/desktop-client/src/commands/license.rs` encrypts the API key) and
  `mask.rs` (truncation/masking, PAN precedent).
- Shift data exists for the analytics follow-up (migration `021_shifts.sql`:
  user_id, terminal_id, opening/closing/expected/cash_difference/total_sales).
- Audit-log infrastructure exists; the registry keys `staff:read_identity`,
  `staff:read_payroll`, `staff:edit_notes` are decided in ADR #35 D2.

## 3. Problem statement

There is no identity, payroll, or emergency-contact data in the system at all,
and no contract for mandatory/optional/sensitive/collectible. ADR #35 D6
defines the contract; this slice implements it with the privacy rules — the
sensitive fields must not ride wildcards, must be encrypted at rest, must be
masked in display, must be read-audited, must not leave the device in sync or
export payloads, and must never be auto-deleted.

## 4. Scope of the slice

### 4.1 Schema

New migration adding to `users`: `date_of_birth`, `phone`, `national_id_type`,
`national_id` (UNIQUE when present), `email` (UNIQUE), `monthly_take_home_minor`,
`emergency_contact_name`, `emergency_contact_phone` (all nullable), plus
optionals `job_title`, `notes`, `address`, `language`, `avatar`,
`tax_id`, `national_id_expires_at`, `emergency_contact_relationship`,
`hire_date`. A `profile_complete` view/state derived from the required set.

### 4.2 Validation (creation)

Field-specific errors: `national_id` per type (ssn 9 / nik 16 digits), email
format, phone E.164, DOB not in the future, `monthly_take_home_minor > 0`.
Creation requires the 9 fields; legacy rows enter the incomplete-profile state.

### 4.3 Sensitive handling

- Grants: `staff:read_identity` / `staff:read_payroll` / `staff:edit_notes`
  enforced on the profile command surface; Auditor excluded (via the 0046/0047
  stack).
- Encryption: `national_id` + `monthly_take_home_minor` encrypted at rest via
  `oz-security` keyring.
- Masking: `national_id` displays last-4 by default; full value only via the
  explicit grant, masked in export/log surfaces.
- Read audit: every read of the three sensitive fields emits an audit event
  (key, user, timestamp); the audit records access, not values.
- Residency: sensitive fields excluded from cloud sync and bulk exports.
- Retention: deactivation never deletes sensitive fields; deletion is an
  explicit compliance decision.

### 4.4 Incomplete-profile semantics

Checkout login works; the user is flagged in staff management; management-role
assignment and sensitive grants require a complete profile.

## 5. Implementation plan

1. Write the Red tests first: validation matrix, incomplete-profile semantics,
   encryption round-trip, masking, read-audit, residency (sync/export
   exclusion), retention.
2. Add the migration + runner registration (Green for schema tests).
3. Implement validation + incomplete-profile state in the staff command
   surface (both clients).
4. Wire the sensitive grants through the 0046/0047 stack.
5. Add encryption (oz-security keyring), masking, read-audit, and residency
   exclusions in the profile read/export/sync paths.
6. Run area tests: `test-tdd.sh -p crates/oz-core`, `cargo test -p oz-pos-app
   --lib`, `cargo test -p oz-pos-tablet --lib`, fmt, clippy, drift guard.

## 6. Test plan

### Existing tests to modify

- `create_staff` / `create_staff_scoped` tests
  (`apps/desktop-client/src/commands/staff.rs` ~692–820, tablet `staff.rs`):
  arg fixtures gain the 9 required fields;
  `create_staff_args_deserialize` / `_debug` pin the new wire shape.
- UI `StaffManagementScreen.test.tsx` create-form fixtures gain the required
  fields; `StaffLoginScreen.test.tsx` / `StaffLoginKeyboard.test.tsx` are
  checked for affected login-path assertions.
- Direct-SQL `users` INSERT fixtures (the
  `('user-owner', 'owner', 'hash', 'Owner', 'role-owner', 1, ...)` rows) keep
  working — columns are nullable — but any command-path call with minimal
  args must add the required fields.

### New tests (Red first)

- Validation matrix: each required field missing or malformed (national ID
  per type, email format, phone E.164, DOB in the future, non-positive
  `monthly_take_home_minor`) → field-specific error.
- Uniqueness: `national_id` (when present) and `email` reject duplicates.
- Incomplete-profile semantics: checkout login works; flagged in staff
  management; management-role assignment and sensitive grants denied.
- Encryption round-trip via `oz-security` keyring; fail-closed when the key
  is missing.
- Masking: `national_id` renders last-4 by default; full value only via the
  explicit grant.
- Read-audit: every read of the three sensitive fields emits an event with
  no values.
- Residency: sensitive fields absent from sync and bulk-export payloads.
- Retention: deactivation never deletes sensitive fields.

## 7. Security and correctness considerations

- Encryption failures fail closed: a missing key never yields plaintext.
- Masked values and audit events never contain the full `national_id`.
- Residency is enforced at the sync/export boundary, not by convention.
- Money stays in i64 minor units, store currency; no floats anywhere.

## 8. Non-goals

- Invitations and org-tenant layer (ADR #35 D7).
- A second PIN/password credential.
- The staff analytics page (ADR #35 D8).
- Bank account, gender/religion/marital status/ethnicity/blood type (the
  D6 not-collected list).
- SQLCipher full-database at rest (open spec C-5).

## 9. Rollback plan

The columns are nullable, so the migration is reversible; the incomplete-
profile state means no legacy row is ever blocked. Each privacy rule ships
with its own test, so a rule that proves operationally wrong (e.g. masking
frustrating a workflow) can be adjusted independently without reverting the
schema.
