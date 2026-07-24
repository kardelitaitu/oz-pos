# OZ-POS Foundation Review — Brutal & Honest

> **Reviewed:** 2026-07-24 · **Reviewer Role:** Senior Technical Lead / Project Manager
> **Scope:** Core architecture, Rust backend, React/TypeScript frontend, testing, DevEx, and strategic trajectory.

---

## Action Checklist

> Work through these in order. The first three are blockers before any merchant goes live.

### 🔴 Critical (Do Before Beta)

- [x] **P1 — Finish `oz-core` modularization** — Extract domain logic & DB persistence out of `oz-core` into each `modules/<name>/src/services/` and `modules/<name>/src/repositories/`. *(All 5 phases complete: sales, inventory, crm, loyalty, staff, terminal, settings, tax, reporting ✅)* *(Month 1–2)*
- [x] **P2 — Fix `App.tsx` self-registration** — Each feature folder exports a `register()` function; `App.tsx` calls `registerAllFeatures()`. Remove all 35+ manual `registerPage()` / `registerNavItem()` calls from `App.tsx`. *(Decentralized UI registration complete across all 24 UI features ✅)* *(Week 1–2)*
- [x] **P3 — Rethink sync conflict strategy** — Replace blanket LWW with entity-specific rules: completed sales → immutable; stock → delta-merge; settings → LWW is fine. *(Completed & tested in platform/sync/src/conflict.rs ✅)* *(Month 2–3)*

### 🟠 Medium (Do This Sprint)

- [x] **P4 — Replace `rlua` with `mlua`** — Drop-in swap. `rlua` has been unmaintained since 2021. *(1 day)*
- [ ] **P5 — Complete the 45-page manual QA walkthrough** — Run the full `TODO.md` checklist on the actual Tauri binary. Only 8/45 pages checked after 6 sprints. *(2 days)*
- [x] **P6 — Clean repository noise** — Add `vitest-profile.json`, `vitest-output.log`, `test-baseline.log`, `test-nextest-baseline.log`, `ui/nul`, `nul` to `.gitignore` and `git rm --cached`. *(1 day)*

### 🟡 Low (Next Sprint)

- [x] **P7 — Decide: React or SolidJS** — Write ADR #30 and delete the migration footnote from `ARCHITECTURE.md`. Ambiguity is costing design decisions. *(Decision + 1 day to document)*
- [ ] **P8 — Audit root context providers** — Determine which of the 9 providers in `App.tsx` genuinely need to be at the root. Move `HardwareAccelContext`, `ZoomContext`, `BrandContext` lower in the tree. *(Half day)*
- [ ] **P9 — Update `ARCHITECTURE.md`** — Fix stale claims: "22+ crates" → 29, "three ADRs" → 29, "9 modules" → 10. *(30 min)*
- [ ] **P10 — Remove committed `nul` files** — Delete `ui/src/nul`, `ui/nul`, root `nul` and add to `.gitignore`. *(30 min)*

---

## Executive Summary

OZ-POS is an **architecturally ambitious** project with a strong theoretical foundation. The design docs,
ADRs, workspace layout, and tooling conventions are well above average for a solo-dev or small-team project.
The *bones are solid*. But there is a significant **execution gap** between the documented architecture and
what actually exists in code — and several structural decisions that will cause serious pain at scale.
This review will not hold back.

---

## Section 1 — What's Actually Good ✅

### ✅ 1. Workspace & Crate Separation is Legitimate Enterprise-Grade

The Cargo workspace with 29+ members across `foundation/`, `platform/`, `modules/`, `crates/`, and `apps/`
is the right call. This is how serious Rust backends are structured. The `resolver = "2"` config,
centralized `[workspace.dependencies]`, and per-package dev overrides (argon2 at `opt-level = 3` in dev)
show real attention to build-time UX.

### ✅ 2. The Money/Currency Value Object is Correct

`Money { minor_units: i64, currency: Currency([u8; 3]) }` is exactly what financial software should use.
No floats. ISO-4217. Currency-aware arithmetic. Stack-allocated. This is the right decision and it is
enforced at the type level — not just a convention. Most fintech projects get this wrong.

### ✅ 3. The Event Bus Architecture is Sound

The synchronous in-process `EventBus` with type-erased `Arc<dyn Fn>` handlers, `RwLock` with
snapshot-before-dispatch (preventing reentrant deadlocks — the Bug #2 fix in code comments), and
module-scoped subscription tracking (`unsubscribe_module`) is well-engineered. The ADR references prove
this was thought through, not hacked together.

### ✅ 4. The Feature Flag System is Comprehensive

32+ toggleable features using a typed `Feature` enum (not stringly-typed keys), persisted as
`feature.<variant>` rows in SQLite. Dependency resolution between features is documented. This is
production-quality feature gating done right.

### ✅ 5. Test Coverage Breadth is Impressive

**202 test files** in `ui/src/__tests__/` alone — covering every major screen, all hooks, accessibility
compliance (`a11y/`, `colorContrastCompliance`, `focusVisibleCompliance`, `touchTargetSizing`), animation
behavior, and IPC contract tests. This is rare at this stage of a project. Most startups write zero tests
until after launch.

### ✅ 6. i18n Infrastructure is Done Right

`@fluent/react`, per-domain `.ftl` files, CI scripts (`verify-bundle-parity.py`, `dedupe-ftl.py`,
`lint-i18n.sh`), pre-commit gate for bundle parity — this is enterprise-grade localization from day one.
Most companies bolt this on painfully after two years.

### ✅ 7. Developer Experience Tooling

Pre-commit hooks (4 gates), `graphify` knowledge graph, `scripts/check.sh`, flamegraph helpers, Criterion
benchmarks, tokio-console integration, Lighthouse CI — you clearly understand that DevEx is a force
multiplier.

---

## Section 2 — The Real Problems 🔴

### 🔴 CRITICAL — `oz-core` is a God Crate in Denial

This is the single biggest structural problem in the entire project.

`crates/oz-core/src/` has **53 source files** containing essentially the entire business domain:

| File | Size |
|------|------|
| `features.rs` | 69,913 bytes (1,887 lines) |
| `location_resolver.rs` | 52,618 bytes |
| `migrations.rs` | 44,622 bytes |
| `subscription.rs` | 33,033 bytes |
| `sync_client.rs` | 36,887 bytes |
| `settings.rs` | 36,777 bytes |
| `sale.rs`, `product.rs`, `inventory.rs`, `loyalty.rs`... | all here |

`oz-core` contains: sales logic, product logic, inventory logic, tax logic, customer logic, gift cards,
KDS, loyalty, promotions, stock counts, stock transfers, shifts, terminals, exchange rates — the entire
business domain in one crate.

The architecture says *"Modules Own Business Logic"* and *"No Direct Module-to-Module Calls"*. But
`oz-core` owns **everything**, and all modules are thin wrappers. If Sales needs `oz-core`, and CRM needs
`oz-core`, they are not decoupled — they share the same physical crate.

**Impact:**
- Compile times blow up — any change to `oz-core` recompiles every downstream crate
- Independent module testing is fiction — they all depend on the same crate
- The business logic boundary exists in documentation, not in the compiler

### 🔴 CRITICAL — `App.tsx` Violates the Registry Pattern Its Own Docs Define

The entire rationale of `platform/ui/page-registry` and `menu-registry` is that modules self-register
their routes. But `App.tsx` manually imports **35+ screens** and calls `registerPage()` / `registerNavItem()`
for all of them in a single 249-line file. This means:

1. Adding any new screen requires touching `App.tsx` — tight coupling to a single file
2. The "feature module owns its frontend" principle is broken at the code level
3. The file mixes three distinct concerns: imports, registrations, and the root `App()` component

The registry pattern exists to make `App.tsx` a 10-line file. It currently is not.

### 🔴 CRITICAL — The Module Implementations are Skeletal

```
modules/sales/src/lib.rs      → 6,073 bytes (1 file)
modules/inventory/src/lib.rs  → similar
modules/crm/src/lib.rs        → similar
```

Every business module should own its backend logic per the stated architecture. But the modules are
lifecycle stubs — the real business logic is all in `oz-core`. The module system is built; the migration
of logic into it is not done. You have built scaffolding for a modular architecture without completing
the modularization.

### 🔴 HIGH — Last-Write-Wins Sync is Wrong for Financial Data

`platform/sync/` implements **Last Write Wins (server-authoritative on tie)** conflict resolution.

For a multi-terminal POS scenario:
- Terminal A applies a 10% discount to a sale
- Terminal B simultaneously voids the same sale
- LWW picks whoever has the later timestamp

For financial data — sales, payments, inventory deductions — LWW is the **wrong conflict strategy**.
This is a data correctness risk in a financial application. You need entity-specific rules:

| Entity | Correct Strategy |
|--------|-----------------|
| Completed sales | Immutable — reject any mutation after completion |
| Stock levels | Delta-based (add/subtract), not absolute value overwrite |
| Settings/preferences | LWW is fine here |
| Void/refund | Always wins over discount/modifier |

### 🟠 MEDIUM — Context Provider Nesting is a Performance Anti-Pattern

```tsx
<ErrorBoundary>
  <LocaleProvider>
    <BrandProvider>
      <ZoomProvider>
        <HardwareAccelProvider>
          <ThemeProvider>
          <CurrencyProvider>
          <AuthProvider>
            <ToastProvider>
              <WorkspaceProvider>
```

Nine nested providers at the root means every top-level state change propagates through the entire tree
unless each provider is carefully memoized. There is also a formatting bug on line 241 —
`</CurrencyProvider></ThemeProvider>` collapsed onto one line — indicating this file has been manually
edited repeatedly without discipline.

Not all of these need to be at the root. `HardwareAccelContext`, `ZoomContext`, and `BrandContext` can
live lower in the tree.

### 🟠 MEDIUM — `rlua` is Abandoned

`Cargo.toml` lists `rlua = "0.20"`. The `rlua` crate has been **unmaintained since 2021**, superseded
by `mlua`. For a Lua scripting integration you are positioning as a plugin/promotions engine, this is a
security and maintenance liability. Lua VM vulnerabilities will not be patched in `rlua`.

### 🟠 MEDIUM — Test Artifacts are Committed to the Repository

The following files are committed and should not be:

| File | Size |
|------|------|
| `vitest-profile.json` | 720 KB |
| `vitest-output.log` | 438 KB |
| `test-nextest-baseline.log` | 381 KB |
| `test-baseline.log` | 302 KB |

This is ~1.8 MB of test output noise in the repository. It will grow with every test run.

### 🟠 MEDIUM — The SolidJS Migration Plan is a Ghost

`ARCHITECTURE.md` marks `React → SolidJS` as a planned migration with `"State Management: Solid Store*"`.
You have since built 202 test files, hundreds of components, and a full hook library for React.
The ambiguity is dangerous — it affects every architectural decision about state management.

**Make a decision and delete the footnote.** Either commit to React 18 with proper `useMemo` /
`useReducer` patterns, or start the SolidJS migration now in a feature branch.

### 🟡 LOW-MEDIUM — Manual QA is at 8/45 Pages After 6 Sprints

`TODO.md` shows only **8 of 45 pages** manually QA'd as of 2026-07-22, after six complete sprints.
The core happy paths — POS Terminal, Payment Modal, Receipt Printing, Settings — are not verified.
Automated tests do not catch visual regressions, missing Fluent keys in real Tauri rendering, or IPC
errors on the actual compiled binary.

### 🟡 LOW — `nul` Files Committed to the Repository

There are `nul` files at `ui/src/nul`, `ui/nul`, and the project root. These are Windows null device
artifacts. They should be removed and added to `.gitignore`.

### 🟡 LOW — `ARCHITECTURE.md` is Already Stale

The document says "22+ crates" (reality: 29), "three ADRs" (reality: 29 in `docs/decisions/`), and
"9 modules" (reality: 10). The audit stamp acknowledges this. A document that is immediately wrong
erodes trust in the documentation as a whole.

---

## Section 3 — Prioritized Recommendations 📋

### Priority 1 — Finish the `oz-core` Modularization (Month 1–2)

This is the most impactful change you can make. The steps:

1. For each business module (`sales`, `inventory`, `crm`, `tax`, `staff`, etc.), create
   `modules/<name>/src/services/` and `modules/<name>/src/repositories/`
2. Move the corresponding `oz-core/src/<domain>.rs` and `oz-core/src/db/<domain>.rs` files there
3. `oz-core` becomes a thin facade: shared DB infrastructure, migration runner, sync client only
4. Modules depend on `foundation` and `platform-core` — not on each other through `oz-core`

This is the restructuring the architecture claims happened. It has not yet happened.

### Priority 2 — Fix `App.tsx` (Week 1–2)

Each feature folder exports a `register()` function:

```typescript
// ui/src/features/sales/register.ts
export function register() {
  registerPage({ route: 'sales', component: PosScreen, label: 'POS Terminal', feature: 'simple-retail' });
  registerNavItem({ route: 'sales', label: 'POS Terminal', feature: 'simple-retail', i18nKey: 'nav-pos-terminal', section: 'operations', icon: posIcon });
}
```

`App.tsx` becomes:

```typescript
import { registerAllModules } from '@/features';
registerAllModules();

export default function App() { ... }
```

One line per module. `App.tsx` shrinks from 249 lines to ~30.

### Priority 3 — Replace `rlua` with `mlua` (1 Day)

Drop-in replacement. `mlua` is API-compatible for most use cases and actively maintained.
Do this before the Lua integration goes deeper.

### Priority 4 — Rethink Sync Conflict Strategy (Month 2–3)

Define explicit conflict rules per entity type in `platform/sync/`. Start with the highest-risk entities
(completed sales, inventory stock) and give them immutability or delta-merge semantics before the first
merchant goes live with multi-terminal mode.

### Priority 5 — Complete the Manual QA Walkthrough (This Sprint)

Stop writing new features. Spend 2 focused days doing the full 45-page walkthrough in the actual running
Tauri binary. Most production bugs are found this way. Tag every failure found as a `fix/<name>` branch
and fix them before any public release.

### Priority 6 — Clean Repository Noise (1 Day)

Add to `.gitignore`:

```
vitest-profile.json
vitest-output.log
test-baseline.log
test-nextest-baseline.log
ui/nul
nul
```

Then run `git rm --cached` to untrack committed artifacts.

### Priority 7 — Decide: React or SolidJS

Call the decision now. Delete the footnote from `ARCHITECTURE.md`. Write an ADR #30. If staying with
React: audit all providers for memoization, adopt `useReducer` for POS screen state. If migrating:
start with `foundation/` and `platform/` layers, not the feature screens.

---

## Section 4 — Scorecard

| Dimension | Score | Notes |
|-----------|-------|-------|
| Architecture Design | 8/10 | Excellent on paper, partial in reality |
| Backend Code Quality | 7/10 | Money / EventBus / Kernel are solid; `oz-core` is a liability |
| Frontend Code Quality | 6/10 | Registry pattern exists but `App.tsx` bypasses it |
| Test Coverage | 8/10 | 202 test files — exceptional for this stage |
| i18n / Accessibility | 9/10 | Best-in-class for an indie project |
| DevEx / Tooling | 8/10 | Pre-commit gates, flamegraph, graphify — impressive |
| Documentation | 7/10 | ADRs exist; docs drift from code |
| Security Posture | 6/10 | Argon2 ✅ · `rlua` unmaintained ❌ · LWW financial risk ⚠️ |
| Sync / Offline Strategy | 5/10 | LWW is wrong for financial entity conflicts |
| **Overall** | **7/10** | Solid foundation, significant execution gaps |

---

## Closing Verdict

You have built the scaffolding for a serious commercial POS system. The architecture document reflects
genuine design thinking. The test suite is impressive. The tooling is professional.

But **the hard work is still ahead**. The modular architecture does not yet exist at the code level — it
exists at the documentation level. `oz-core` is doing the work that 10 individual modules should be
doing. `App.tsx` knows about every screen in the system. The sync strategy is wrong for your domain.

None of these problems are fatal. They are all fixable. But they must be fixed **before** the first
merchant goes live with real financial data. Once you have live sales records, customer accounts, and
inventory in production, refactoring `oz-core` becomes orders of magnitude riskier.

The foundation is strong enough to build on. It is not strong enough to ship to paying customers without
addressing the three critical issues above first.

---

*This review was generated by code analysis of the actual source tree — not the documentation alone.
Every finding is traceable to a specific file and line range in the codebase.*
