# RBAC centralized fail-closed enforcement gate

## 1. Decision requested

Introduce the single backend `require_permission(permission)` gate that ADR
#35 D3 requires, migrate every permission-sensitive command onto it, and pin
the set of gated commands with a test so a new command that skips the gate is
caught by the suite. This is D9 step 2 and depends on the 0046 registry.

## 2. Evidence baseline

- Enforcement today is per-command `require_permission_for_user(store,
  user/role, permission)` — e.g. `apps/desktop-client/src/commands/authz.rs`,
  `customers.rs`, `exchange_rates.rs`, `loyalty.rs` (desktop and tablet).
- Rounds 172 and 174 found real gaps of exactly this class: `list_customers_scoped`
  resolved the session but skipped the `customers:view` gate (CRM-02), and
  `create_exchange_rate` skipped field validation entirely (CUR-05). The fixes
  landed per-command; the class of "command forgot its gate" is not prevented.
- Frontend gating exists in places and is presentation only.

## 3. Problem statement

A permission model is only as strong as its weakest command. With per-command
gates, "did every command gate itself?" is answered by review and by audit —
both of which missed instances. A single gate plus a pinned gated-command set
turns the question into a compile/test-time guarantee and gives ADR #35's
fail-closed rule one place to live.

## 4. Scope of the slice

### 4.1 The gate

`require_permission(permission)` on the backend resolves the caller's role
through the 0046 registry (family wildcards, sensitive keys, `"*"`), and denies
by default: an unregistered permission or an unresolvable role fails closed.

### 4.2 Command migration

Every existing `require_permission_for_user` call site moves onto the gate.
No command behavior changes; only the enforcement channel does.

### 4.3 Pinned gated-command set

A test enumerates the permission-sensitive commands and asserts each passes
through the gate. The list is explicit and reviewed — a new command must be
added to it, which is the review signal.

## 5. Implementation plan

1. Add the gate on top of the 0046 registry with deny-by-default semantics.
2. Write the failing test: a command that skips the gate is detected (Red).
3. Migrate the command call sites (Green), keeping round-172/174 tests green.
4. Add the pinned gated-command enumeration test.
5. Run area tests: `cargo test -p oz-pos-app --lib`, `cargo test -p oz-pos-tablet
   --lib`, `test-changed.sh`, fmt, clippy on both clients.

## 6. Test plan

### Existing tests (migration contract — must stay green unmodified)

- Round-172 `customers.rs` tests (desktop + tablet) and round-174
  `exchange_rates.rs` tests (desktop + tablet): they assert observable command
  behavior, so re-pointing the enforcement channel must not touch them.
- `commands::authz` unit tests — re-pointed at the gate if any assert on
  `require_permission_for_user` directly (implementation detail).

### New tests (Red first)

- Deny-by-default: an unregistered permission key denies; an unresolvable
  role denies.
- Pinned gated-command set: a test enumerates every permission-sensitive
  command and fails for one that skips the gate — a new ungated command is
  caught by the suite.
- The gate resolves family wildcards and sensitive keys through the 0046
  registry.

## 7. Security and correctness considerations

- The gate must be the only authorizer: a frontend role check never gates a
  backend pass, and no second parallel enforcement path is introduced.
- Resolution errors deny, never allow.
- Session resolution stays where it is today; this slice only centralizes the
  permission check (assignment scopes arrive in 0048).

## 8. Non-goals

- Branch/workspace assignment scopes (0048).
- Profile fields (0049) and caching (ADR #35 D7).
- UI redesign beyond removing frontend-only gating that exists.

## 9. Rollback plan

The gate wraps the existing checks; reverting it restores the per-command
pattern without data or schema impact. The pinned command-set test is the
canary — if it flags false positives (a gated command that was already
internal), the list is corrected deliberately, not broadened silently.
