# Development Checklist — OZ-POS Desktop

A checklist to ensure the app is fully functional during development. Run through this after any DB migration, subscription tier change, or dev-mock update.

---

## 1. First-Run Bootstrap Flow

The app must handle a **fresh database** gracefully.

- [ ] **`has_users` IPC exists** — `apps/desktop-client/src/commands/auth.rs` exposes a `has_users` command that returns `{ has_users: bool }` by checking `Store::list_users()`.
- [ ] **Registered in invoke_handler** — `has_users` is listed in `lib.rs` `generate_handler![]`.
- [ ] **UI wrapper exists** — `ui/src/api/staff.ts` exports `hasUsers()`.
- [ ] **AppShell checks on startup** — `AppShell.tsx` calls `hasUsers()` and shows `CreatePinScreen` when `hasAnyUsers === false`, bypassing the license/setup gate.
- [ ] **dev-mock has handler** — `dev-mock/tauri-api.ts` returns `{ has_users: true }` (mock always has seeded staff).
- [ ] **`bootstrap_owner` works end-to-end** — `CreatePinScreen` → `bootstrap_owner` → argon2 hash → user row created → `swapSession` → user lands on workspace picker.

## 2. Subscription Tier & Feature Gates

Features are gated by the `get_subscription_capabilities` IPC. The dev database starts with a **Free** tier bootstrap row.

### 2a. Rust Backend (`subscription.rs`)

- [ ] **Debug override** — In `#[cfg(debug_assertions)]`, Free tier is upgraded to Premium so all features work during `cargo tauri dev`.
- [ ] **All capability fields use the overridden `tier`** — Every field (`supportsAnalytics`, `supportsLoyalty`, `supportsQris`, `supportsDailyDashboard`, etc.) must read from the overridden `tier` variable, NOT from the original `sub` object. (Previously `supports_analytics` used `sub.supports_analytics_with_addons()` which bypassed the override.)
- [ ] **Quota fields use the overridden tier** — `max_stores`, `max_pos_instances`, `max_warehouses`, `max_staff_users`, `sales_history_days`, `offline_grace_days` all come from `tier.method()`.

### 2b. Dev-Mock (`dev-mock/tauri-api.ts`)

- [ ] **`get_subscription_capabilities` exists** — Returns Premium-tier caps:
  ```ts
  'get_subscription_capabilities': () => ({
    tier: 'premium', maxStores: null, maxPosInstances: null,
    maxWarehouses: null, maxStaffUsers: null, salesHistoryDays: null,
    supportsQris: true, supportsAnalytics: true, supportsLoyalty: true,
    supportsDailyDashboard: true, supportsCloudSync: true,
    offlineGraceDays: 30, storeCount: 1, staffCount: 1, terminalCount: 1,
    addons: [],
  }),
  ```
- [ ] **`get_license_status` returns active** — Returns `{ isActive: true, status: 'valid', tier: 'pro', ... }`.
- [ ] **`check_license_status` returns active** — Returns `{ status: 'active', tier: 'Pro', active: true, ... }`.

### 2c. UI Feature Gates (5 screens)

| Screen | Gate check | Unlocked when |
|--------|-----------|---------------|
| `AnalyticsScreen.tsx` | `caps && !caps.supportsAnalytics` | `supportsAnalytics: true` |
| `LoyaltyManagementScreen.tsx` | `caps && !caps.supportsLoyalty` | `supportsLoyalty: true` |
| `DailyTotalWidget.tsx` | `caps && !caps.supportsDailyDashboard` | `supportsDailyDashboard: true` |
| `SetupWizard.tsx` | `!!caps && !caps.supportsQris` | `supportsQris: true` |
| `PaymentModal.tsx` | `caps && !caps.supportsQris` | `supportsQris: true` |

**Key behavior**: When `caps` is `null` (loading/error), `caps && !caps.supportsX` evaluates to `false` — features render **open**, not locked. This is the correct fallback.

### 2d. Quota Limit Checks (3 screens)

| Screen | Gate check | Unlocked when |
|--------|-----------|---------------|
| `TerminalManagementScreen.tsx` | `caps.terminalCount >= caps.maxPosInstances` | `maxPosInstances: null` (unlimited) |
| `TopologyScreen.tsx` | `caps.storeCount >= caps.maxStores` | `maxStores: null` (unlimited) |
| `StaffManagementScreen.tsx` | `caps.staffCount >= caps.maxStaffUsers` | `caps.tier === 'premium'` check for approaching limit |

## 3. AppShell Startup Flow

```
DEV mode:
  hasCompletedSetup = true, hasActiveLicense = true
  hasUsers() → false → Show CreatePinScreen
  hasUsers() → true  → Show StaffLoginScreen

Production:
  getSetupStatus + getLicenseStatus
  hasUsers() → false → Show CreatePinScreen (first-run)
  hasUsers() → true  → Check license → SetupWizard or StaffLoginScreen
```

- [ ] **DEV bypass does not skip user check** — `hasUsers()` is called even when `hasCompletedSetup = true`.
- [ ] **Production path checks users** — `hasUsers()` is called in the `Promise.all` startup block.

## 4. Dev-Mock Completeness

The dev-mock (`ui/src/dev-mock/tauri-api.ts`) must cover every IPC command the UI calls. Commands missing from the mock cause silent failures in browser preview.

### Commands in Rust but **missing from dev-mock** (3 — all internal-only):

**Internal-only (never called by UI — safe to skip):**
- [ ] `recover_pending_topology_apply_at_startup` — startup topology recovery
- [ ] `settings_changed_sink` — internal event bridge
- [ ] `recover_workspace_instances_scoped` — workspace recovery

## 5. Quick Verification Script

After making changes, run this mental checklist:

```bash
# 1. Rust compiles
cargo check -p oz-pos-app

# 2. UI tests pass
cd ui && npx vitest run

# 3. App starts fresh
#    - Delete or rename oz-pos.db to test fresh DB flow
#    - App should show CreatePinScreen (not StaffLoginScreen)
#    - Bootstrap owner with username "owner" / PIN "1234"
#    - After bootstrap, workspace picker appears with demo workspaces

# 4. Features accessible
#    - Analytics screen shows charts (not "Pro feature" lockout)
#    - Loyalty screen shows tier management (not locked)
#    - Daily Sales Dashboard renders (not blurred teaser)
#    - QRIS toggle available in Setup Wizard

# 5. Login works
#    - Username "owner" / PIN "1234" logs in
#    - Rate limiter locks after 3 failed attempts
#    - Session creates and workspace selection works
```

## 6. Common Pitfalls

| Pitfall | Root cause | Fix |
|---------|-----------|-----|
| "Analytics is a Pro feature" on fresh install | `get_subscription_capabilities` returns Free tier; `supports_analytics` reads from original `sub` object instead of overridden `tier` | Use `tier.supports_analytics()` not `sub.supports_analytics_with_addons()` |
| Login fails with "invalid username or PIN" | Owner account doesn't exist; `CreatePinScreen` was never shown because DEV mode skips activation flow | Add `has_users` check in AppShell; show `CreatePinScreen` when no users exist |
| Browser preview shows locked features | `get_subscription_capabilities` missing from dev-mock; `caps` stays `null` but some gates don't handle null correctly | Add `get_subscription_capabilities` to dev-mock returning Premium caps |
| Workspace picker shows "No workspaces" | `list_workspaces` fails because picker ticket is invalid or DB has no workspace instances | Fallback to `FALLBACK_WORKSPACES` (already implemented in WorkspaceContext) |
| `platform-sync` won't compile | `PgTransport::new` signature changed (added `tenant_id`) but call site not updated | Pass `tenant_id` to `PgTransport::new` in `pg_daemon.rs` |
| Warehouse workspace shows old product list | Workspace-to-route mapping still points to `inventory` instead of `warehouse` | Change `warehouse: 'inventory'` → `warehouse: 'warehouse'` in AppShell |
| Home screen opens wrong page (e.g. analytics instead of settings) | Stale `#/analytics` hash from shortcut persists across workspace switches | Clear hash after consuming it in AppShell workspace routing effect |
| Home screen looks like old settings page | WorkspaceHome renders stale analytics/reports shortcuts inline instead of using tools section | Ensure WorkspaceHome uses `.workspace-home-content` with two `.workspace-section` divs (Workspaces + Tools) |
