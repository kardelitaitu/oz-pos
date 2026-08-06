<!-- Audit stamp: 2026-07-29 · Codebuff · status: UPDATED — July 29 full-codebase i18n audit session appended -->

# OZ-POS Development Journal

## 2026-08-06 — TDD cycle: tablet first-owner bootstrap + device binding (audit/06 parity)

### The tablet could not self-provision and could never auto-boot into a bound workspace
**Problem:** Two gaps from the parity audit: (1) the tablet registered no `bootstrap_owner`, so a fresh tablet had no way to create the first owner; (2) `resolve_boot_store` always returned the primary store — the tablet had no keyring/binding surface (`DEVICE_BINDING_KEYRING_NAME`, `sign_binding`, `set_device_binding*`), so a bound tablet could never auto-boot into its store+instance.

**Solution:** Red→Green TDD cycle, mirroring the desktop client. (1) `bootstrap_owner` in tablet `staff.rs` (args/result/command + extracted `run_bootstrap_owner`): validates fields + PIN length, fails closed when users exist, seeds default roles, creates the owner, and the command mints the picker ticket bound to the new owner (9 tests incl. command-level mint). (2) Binding surface in tablet `terminals.rs`: `DEVICE_BINDING_KEYRING_NAME`, `sign_binding` (HMAC-SHA256 over `{terminal}:{store}:{instance}` with OS-keyring secret), `SetDeviceBindingArgs`, `set_device_binding` / `set_device_binding_scoped` (6 tests, incl. FK fixture). (3) `resolve_boot_store` rewritten in tablet `workspaces.rs` with `verify_binding_hmac` (constant-time `verify_slice`) + extracted `resolve_boot_store_core(conn, db_manager, device_id, keyring)` for deterministic tests: auto-boot, tampered binding → primary, missing instance → primary, unknown device → primary, plus HMAC unit tests (7 tests). (4) Frontend: `WorkspaceContext` now passes `getDeviceId()` into `resolve_boot_store` — verified both clients' `get_device_id` return exactly the backend env fallback (`COMPUTERNAME || HOSTNAME`), so desktop binding behavior is unchanged (2 new frontend tests).

**Send-safety note:** `Box<dyn Keyring>` is non-`Send`; clippy caught the command futures holding it across `db.lock().await`. Fixed by acquiring the keyring only AFTER the lock — matches the repo's established pattern.

**Verify:** tablet lib 414/414 (24 new) · frontend WorkspaceContext 24/24 + typecheck · eslint clean · fmt clean · clippy clean on the tablet (no remaining errors).

**Shared-checkout sweep:** another thread's `b2700ed1` swept my `staff.rs`/`lib.rs`/partial `terminals.rs`+`workspaces.rs` into its MONEY-06 commit mid-cycle; my post-sweep fixes (Send-safety, FK fixtures, `verify_binding` removal) were committed separately, and HEAD was momentarily non-compiling (lib.rs registered `set_device_binding` before Cargo.toml had `oz-security`).

**Deliberately NOT done (follow-ups):** (1) the tablet shell still never renders `CreatePinScreen` — `bootstrap_owner` is registered + tested for parity/standalone provisioning, but a fresh tablet remains desktop-provisioned until a first-run UI is wired. (2) Bindings can only be created via `set_device_binding*` — no tablet screen calls them yet (desktop's terminal-management screen is the UI today); note the tablet stores bindings in the GLOBAL DB while the desktop scoped variant writes the session store DB — consistent with the tablet's global-only terminal read path, but revisit if terminal/binding records ever sync from desktop store DBs. (3) On mobile without an OS keyring, `default_keyring` degrades to `InMemoryKeyring`, so bindings don't survive restarts on such devices — inherent platform limitation, same as desktop's dev fallback.

## 2026-08-06 — TDD slice: tablet vs desktop pre-session auth surface (audit/06 parity audit)

### Comparison result: the tablet now shares the hardened picker AND session-mint surface — no gaps remain
**Prompt:** run a TDD slice comparing the tablet client's pre-session auth surface against the hardened desktop commands.

**Evidence (command-by-command diff of `apps/*-client/src/lib.rs` registrations + command bodies):**

| Pre-session surface | Desktop | Tablet | Verdict |
|---|---|---|---|
| `staff_login` (PIN verify + mints picker ticket) | ✓ | ✓ (b10f4929) | parity — both mint `user_id.expiry.hmac`, 5-min TTL, per-process secret |
| `bootstrap_owner` (first-owner) | ✓ registered | ✗ not registered | deliberate — tablet shell (`TabletAppShell`) never imports `CreatePinScreen` / never calls `bootstrapOwner`; tablet is a paired device provisioned from the desktop |
| `create_session` (session mint, `verify_instance_access` fail-closed gate) | ✓ | ✓ | parity — identical `role_id`/`user_id`/`instance_id`/`store_id` gate, real role resolved from DB |
| `list_workspaces` (ticket → real user+role → store listing) | ✓ | ✓ | parity — identical body (verify ticket → resolve user/role from global DB → `Store::list_workspaces(real_role, user, store)`) |
| `list_workspace_screens` (ticket-gated bootstrap read) | ✓ | ✓ | parity |
| `resolve_boot_store` | ✓ device-binding + primary fallback | ✓ primary fallback only | deliberate difference — tablet has no device-binding keyring, `is_bound` is always `false` (documented in the command doc) |

**Frontend contract traced end-to-end (why the empty state can only mean a null ticket):** `AuthContext.login` stores `result.picker_ticket`; `CreatePinScreen` bootstrap passes `result.picker_ticket` through `swapSession(session, ticket)`; `WorkspaceContext.fetchWorkspaces` returns early when `pickerTicket` is null (→ `WorkspaceHome` empty state) and falls back to demo cards on empty/error listings. So the screenshot's `No workspaces available` was the pre-fix tablet (no ticket minted) — closed by b10f4929.

**Verify:** tablet `commands::auth` 13/13 + `commands::workspaces` 7/7 · desktop `commands::auth` 19/19 + `commands::workspaces` 17/17 — all parity regression tests green on both clients. `swapSession` optional-ticket path (FastPINOverlay hot-swap, mid-workspace) intentionally bypasses the picker, so no null-ticket picker path remains.

**Follow-ups:** (1) `bootstrap_owner` absence on the tablet is by design but UNTESTED as a guarantee — a registration-level test asserting the tablet surface contains exactly the documented command set would pin it against accidental drift. (2) The tablet never implements device binding, so `resolve_boot_store` always reports `is_bound: false`; if tablets are ever expected to auto-boot into a bound workspace, the binding HMAC + keyring slice is the gap to close.
