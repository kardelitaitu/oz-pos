# Staff Module Audit — July 2026

> **Audit date:** 2026-07-31  
> **Sector:** Staff module — staff accounts, roles, permissions, PIN authentication, workspace access, and shift identity  
> **Status:** AUDITED · findings require remediation  
> **Production code changed:** None

## Scope

This audit covers staff management UI and API contracts, desktop staff/auth/workspace commands, user and role persistence, RBAC presets, login throttling and session creation, workspace assignment, shift identity, localization, theming, accessibility, performance, and focused tests.

Inspected areas:

- `ui/src/features/staff/StaffManagementScreen.tsx`
- `ui/src/features/staff/StaffManagementScreen.css`
- `ui/src/api/staff.ts`
- `ui/src/api/workspaces.ts`
- `ui/src/__tests__/StaffManagementScreen.test.tsx`
- `ui/src/__tests__/StaffLoginScreen.test.tsx`
- `ui/src/__tests__/StaffLoginKeyboard.test.tsx`
- `apps/desktop-client/src/commands/staff.rs`
- `apps/desktop-client/src/commands/auth.rs`
- `apps/desktop-client/src/commands/workspaces.rs`
- `apps/desktop-client/src/commands/authz.rs`
- `crates/oz-core/src/db/staff.rs`
- `crates/oz-core/src/db/shifts.rs`
- `platform/core/src/rbac.rs`
- `modules/staff/src/{lib,models,repository,service}.rs`
- English and Indonesian Staff Fluent bundles

The review uses the universal audit lenses from `audit/AUDIT_JULY_2026.md`: functionality, state and UX, accessibility/i18n, theming, performance, security/data integrity, and quality assurance.

## Architecture summary

The Staff module is transitional. `modules/staff` provides module registration, domain models, and a minimal repository/service lookup surface, while production CRUD, authentication, workspace assignment, and Tauri command behavior remain in `oz-core` and `apps/desktop-client`. The UI uses the legacy unscoped staff and workspace commands and sends `caller_user_id` values from the frontend.

Built-in roles are seeded in `platform/core/src/rbac.rs`: Owner has `*`; Manager and Staff have broad operational permissions including `staff:create` and `staff:update`; Cashier has sales/shift permissions but no staff-management permissions; Custom has none.

PINs are hashed through the platform auth helper. Login attempts are persisted in `login_attempts`, limited to three failed attempts per username in a 60-second window, and cleared on successful login. A successful login returns user/role information; a later workspace-selection step creates an opaque session token.

## Findings

### STAFF-01 — Staff authorization trusts a client-supplied caller ID instead of the authenticated session (P0)

**Evidence:** `CreateStaffArgs`, `UpdateStaffArgs`, and the legacy workspace assignment APIs contain `caller_user_id` fields. `StaffManagementScreen` fills them from `session?.user_id`, but `create_staff` and `update_staff` authorize using that request field via `require_permission_for_user`. The legacy workspace commands likewise accept raw caller/user IDs, while the scoped workspace variants resolve the caller from `session_token`.

**Impact:** The backend has no cryptographic binding between the IPC request and the claimed caller identity on these legacy staff-management paths. A caller able to invoke the command can substitute another known user ID, such as an owner or manager, and pass the permission check. This is an authorization bypass, not merely a UI role-gating issue.

**Recommendation:** Add session-scoped staff CRUD and role reads/writes. Resolve the caller from the opaque session token and never accept caller identity from the frontend. Migrate `StaffManagementScreen` and workspace assignment calls to scoped APIs; retain legacy commands only as explicitly restricted/deprecated compatibility wrappers.

**Priority:** P0 — direct staff and permission-management authorization risk.

---

### STAFF-02 — Managers and Staff can assign an Owner role without hierarchy protection (P1)

**Evidence:** The Staff screen presents every role returned by `listRoles()` in the role selector. The desktop `create_staff` and `update_staff` commands check only `STAFF_CREATE` or `STAFF_UPDATE`, then pass the requested `role_id` directly to `Store::create_user` or `Store::update_user`. Manager and Staff presets both include `staff:create` and `staff:update`, while the Owner role grants `*`.

**Impact:** Even after caller identity is session-bound, a user with ordinary staff-management permission may create or promote an account to Owner unless another layer not present in the inspected commands blocks it. This can grant global access to settings and all operational domains.

**Recommendation:** Enforce role-assignment policy on the backend: only Owner (or a dedicated role-management permission) may create/promote Owner or modify permission-bearing roles. Prevent self-promotion and last-owner removal/deactivation. Return a structured permission error and add tests for Manager/Staff attempts to assign Owner.

**Priority:** P1 — privilege escalation.

---

### STAFF-03 — Editing a PIN is presented in the UI but is not sent or persisted (P1)

**Evidence:** The edit modal renders a “New PIN” password field and stores it in `form.pin`. The `UpdateStaffArgs` TypeScript and Rust types contain no PIN field, and `handleSave` calls `updateStaff` without `form.pin`. `Store::update_user` updates username, display name, role, and active state only; it never updates `pin_hash`.

**Impact:** Operators receive no indication that a requested PIN change was ignored. A compromised or shared PIN cannot be rotated through the staff-management flow, and the UI’s security affordance is misleading.

**Recommendation:** Either remove the edit PIN field until supported or implement an explicit PIN-reset path that validates the new PIN, hashes it server-side, requires an appropriate permission, invalidates relevant sessions, and tests successful and failed rotation. Do not accept a plaintext PIN beyond the command boundary where it is immediately hashed.

**Priority:** P1 — security control is visibly present but functionally broken.

---

### STAFF-04 — Staff CRUD and legacy workspace reads/listing remain global-database commands (P1)

**Evidence:** `list_staff` and `list_roles` lock `state.db` directly and have no visible permission check. `create_staff` and `update_staff` also lock `state.db` directly and accept caller identity from request data. `list_all_workspaces`, `get_user_workspaces`, and related legacy assignment commands do the same. The UI calls these legacy functions (`listStaff`, `listRoles`, `listAllWorkspaces`, `getUserWorkspaces`, and `setUserWorkspaces`) rather than the available session-scoped workspace APIs.

**Impact:** In a multi-store deployment, staff and role data can be read or mutated outside the session’s resolved store. Workspace data may similarly cross store boundaries. This also makes the authorization issue in STAFF-01 more consequential because the claimed caller is evaluated against the global database.

**Recommendation:** Provide store-scoped staff/role commands and use `session_token` for all reads and writes. Ensure target user IDs and workspace keys are checked within the resolved store, and add two-store isolation tests for listing, creation, update, and assignment.

**Priority:** P1 — tenant isolation and operational data integrity.

---

### STAFF-05 — Workspace assignment is a two-command partial-write flow (P2)

**Evidence:** In edit mode, `handleSave` first awaits `updateStaff`, then separately awaits `setUserWorkspaces`. If the second call fails, the staff record is already changed while the workspace assignment is not. The UI catches the error and leaves the modal error visible, but does not roll back the first mutation or communicate that the account is partially updated.

**Impact:** A staff member can be left with a new role/name but old or missing workspace access. Retrying can produce confusing state and makes administrative changes difficult to audit.

**Recommendation:** Expose one backend transaction for user fields plus workspace assignments, or implement an explicit compensating update with a clear partial-success result. Prefer the session-scoped instance assignment API and validate every requested workspace belongs to the resolved store.

**Priority:** P2 — consistency and recovery.

---

### STAFF-06 — Username pre-check enables account enumeration (P2)

**Evidence:** `staff_check_username` returns separate `found` and `is_active` booleans before PIN entry. The login UI uses these values to display “User not found” versus “Account is deactivated.” The command is callable before authentication and is not part of the failed-login limiter shown in `staff_login`.

**Impact:** An unauthenticated caller can probe which usernames exist and which accounts are disabled. This leaks staff directory information and gives attackers a reliable account-discovery oracle.

**Recommendation:** Return one generic pre-auth response or remove the pre-check and let the login endpoint provide a uniform failure. If the staged UX is retained, rate-limit and audit it, avoid distinguishing nonexistent from inactive accounts, and keep detailed reasons in server logs only.

**Priority:** P2 — authentication information disclosure.

---

### STAFF-07 — Login throttling is keyed by username and can be used to lock out legitimate staff (P2)

**Evidence:** `record_login_attempt` stores attempts by username and locks after three failures within 60 seconds. The login flow records an attempt before verifying whether the username exists or whether the PIN is valid. The username-check endpoint is separate and does not consume this limiter.

**Impact:** Anyone who knows a username can deliberately submit three bad PINs and temporarily prevent that staff member from logging in. There is no inspected per-device/IP/global limiter or administrative unlock path in the Staff API.

**Recommendation:** Combine per-account throttling with device/IP and global abuse controls, use exponential backoff rather than a short hard lock where appropriate, audit lockouts, and provide a secure owner recovery mechanism. Keep error responses uniform.

**Priority:** P2 — availability/security trade-off.

---

### STAFF-08 — Staff load errors are swallowed with no error or retry state (P2)

**Evidence:** `StaffManagementScreen.load` catches the primary staff/role request with only `// IPC unavailable.` and always clears `loading`. Workspace loading errors are also swallowed, leaving the workspace column empty. The screen therefore renders an empty state or an incomplete table without distinguishing a successful empty result from an IPC/database failure.

**Impact:** An administrator may believe there are no staff members, roles, or workspace assignments and make unsafe configuration decisions. Operators cannot retry without navigating away or triggering another unrelated refresh.

**Recommendation:** Add localized error state, retry action, and separate status for staff/role/workspace data. Preserve existing rows when a secondary workspace request fails and show an explicit “workspace data unavailable” state rather than silently using an empty list.

**Priority:** P2 — operational UX and configuration safety.

---

### STAFF-09 — Editing an inactive staff member unintentionally reactivates the account (P1)

**Evidence:** `StaffManagementScreen.openEdit` does not preserve `member.is_active` in the form state. The edit path in `handleSave` always sends `is_active: true` to `updateStaff`, regardless of whether the selected staff member is currently inactive.

**Impact:** Editing an inactive account’s display name, role, or workspace settings silently re-enables login access. An administrator can therefore restore a deactivated account without choosing Restore or receiving a warning.

**Recommendation:** Preserve the current active state in edit form state and send it unchanged for profile edits. Keep activation/deactivation as an explicit, separately authorized action, with regression coverage proving that editing an inactive member does not reactivate it.

**Priority:** P1 — account access-control correctness.

---

### STAFF-10 — Deactivate/restore has no confirmation and allows high-impact account changes with one click (P2)

**Evidence:** The table’s Deactivate/Restore buttons immediately call `toggleActive`, which sends `update_staff`. There is no confirmation dialog, per-row pending/disabled state, or protection in the UI against deactivating the current/last Owner. The backend update command also has no visible target-role or last-owner policy.

**Impact:** An accidental click can disable an account. Combined with the missing role hierarchy and PIN-reset path, staff recovery can become dependent on direct database or bootstrap intervention.

**Recommendation:** Confirm deactivation with the staff name and consequences, disable the action while pending, prevent self/last-owner deactivation in backend policy, and provide an auditable recovery flow.

**Priority:** P2.

---

### STAFF-11 — Several interactive labels and table content are not fully localized (P3)

**Evidence:** The component contains hardcoded English fallback strings such as `No staff members yet.`, `Add your first staff member`, and `Failed to load workspace settings`. It also renders a hardcoded `aria-label="Actions"` fallback and uses a literal em dash for empty workspace access. The Indonesian bundle contains the relevant management keys, but it has a legacy block with a different key set and does not make the component’s hardcoded fallbacks unnecessary when lookup fails.

**Impact:** Missing or malformed localization data produces mixed-language staff administration UI and can hide bundle drift. The table’s accessibility labels can also revert to English in a non-English locale.

**Recommendation:** Use a required-localization helper or shared localized empty/error components, remove user-visible English fallbacks from JSX, and add bundle-parity tests for the full Staff management key set in every supported locale.

**Priority:** P3.

---

### STAFF-12 — Compact action controls and hidden native inputs need touch/a11y verification (P3)

**Evidence:** `.staff-mgmt-action-btn` uses compact padding without an explicit minimum height. Custom radio and checkbox controls hide the native inputs with zero dimensions and rely on `:has()` pseudo-elements for visual state. The surrounding labels are clickable and have focus styling via `:has(...:focus-visible)`, but the implementation should be verified on the supported browser/webview matrix and at coarse-pointer sizes.

**Impact:** Table actions may be difficult to activate on touch terminals, and a browser/webview without the expected `:has()` support could leave custom controls visually inconsistent. Keyboard focus behavior depends on the hidden native controls remaining focusable in the target runtime.

**Recommendation:** Apply the shared touch-target minimum to action buttons and custom-control rows, verify keyboard focus and checked-state styling in the Tauri webview and tablet browser, and add automated touch/a11y coverage for the Staff stylesheet.

**Priority:** P3.

---

### STAFF-13 — Transitional module ownership and tests do not cover production security boundaries (P2)

**Evidence:** `modules/staff/src/lib.rs` documents that CRUD, auth commands, frontend, and API remain in legacy locations. The module repository/service exposes only `get_user` and `get_role`, while production command behavior is in desktop commands and `oz-core`. The focused tests are strong on UI rendering and serialization, but the recorded suites do not exercise session-bound staff mutations, two-store isolation, role hierarchy, PIN rotation, workspace partial failure, enumeration resistance, or last-owner invariants.

**Impact:** The module boundary can drift from the production path, and the most important Staff risks are not protected by integration tests. Passing unit tests may therefore provide false confidence around authorization and account recovery.

**Recommendation:** Clarify or complete the module migration, add command-level integration tests with real session contexts, and cover the security invariants before expanding staff features.

**Priority:** P2 — architecture and assurance gap.

## Positive observations

- PINs are hashed through the platform authentication helper and are not included in the staff DTO returned to the UI.
- Login attempts persist in the database, are cleared after successful login, and include a retry-after message during lockout.
- `require_permission_for_user` resolves the user’s actual role from the database rather than trusting a role ID, which is the correct primitive once caller identity comes from a session.
- Built-in role presets are explicit and tested; Cashier does not receive staff-management permissions, and Custom starts with no permissions.
- The UI provides a loading skeleton, empty state, add/edit modal, inline validation, save failure feedback, active/inactive status, and workspace access controls.
- Workspace assignment replacement itself uses a transaction in the store layer.
- Shift opening validates that the user exists, is active, has a non-negative opening balance, and does not already have an open shift.
- Shift closing reads and persists reconciliation totals inside a transaction, with focused tests for cash differences, payouts, payment methods, refunds, and closed/open behavior.
- The inspected Staff CSS uses design tokens rather than hardcoded theme colors and includes visible focus styling for inputs and custom controls.

## Recommended implementation order

1. **STAFF-01/STAFF-04:** Replace caller-ID/global staff commands with session-scoped, store-resolved commands and backend permission checks.
2. **STAFF-02/STAFF-09/STAFF-10:** Enforce role hierarchy, preserve inactive state during edits, add last-owner protection, and make account activation explicit.
3. **STAFF-03:** Remove or fully implement PIN rotation with hashing, session invalidation, and tests.
4. **STAFF-05:** Make staff profile and workspace assignment atomic.
5. **STAFF-06/STAFF-07:** Harden pre-auth enumeration and lockout/abuse controls.
6. **STAFF-08/STAFF-11:** Add retry/error states and remove localization fallbacks.
7. **STAFF-12/STAFF-13:** Finish touch/a11y verification and add security-focused integration coverage.

## Validation

- Focused Staff UI tests: **48 passed across 3 files** (`StaffManagementScreen`, `StaffLoginScreen`, `StaffLoginKeyboard`)
- `cargo test -p modules-staff`: **9 unit tests passed**
- `modules-staff` doctests: **1 passed**
- Filtered `oz-core` Staff tests: **33 passed**
- `git diff --check -- audit`: passed

Some focused UI runs emitted non-fatal mock warnings for an unhandled `get_brand_settings` command; the test suites still passed.

## Status

This is an evidence-based audit report only. No production code was changed. The report is currently uncommitted in the existing `audit/` folder.
