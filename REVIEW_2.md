# OZ-POS Codebase Review #2 — Brutal & Honest

> **Reviewed:** 2026-07-25 · **Reviewer Role:** Senior Technical Lead / Project Manager
> **Scope:** Full codebase re-review after completion of P1–P10 from FOUNDATION_REVIEW.md.
> **Codebase size:** 281,538 lines · 1,374 files · 29 Rust workspace crates · 203 UI test files

---

## Action Checklist

> Work through these in priority order. Items marked 🔴 are blockers before any merchant goes live.

### 🔴 Critical (Do Before Beta)

- [ ] **R1 — Remove the committed private updater key** — `oz-pos-updater.key` is tracked in git despite being listed in `.gitignore`. Rotate the key pair, purge from git history with `git filter-repo` or BFG, and store the private key exclusively in GitHub Secrets / a secrets manager. *(2 hours)*
- [ ] **R2 — Finish extracting `oz-core/src/db/` into modules** — `oz-core/src/db/sales.rs` is **142 KB / 3,523 lines**. The P1 modularization moved domain *models* but left the entire SQLite transaction layer behind. Until DB access lives in `modules/<name>/src/repositories/`, `oz-core` remains a god crate and compile times blow up on every business-logic change. *(Month 1)*
- [ ] **R3 — Add a production guard to `DevToolbar`** — `ui/src/features/design/DevToolbar.tsx` is unconditionally mounted in `App.tsx` with no `import.meta.env.DEV` guard. Developer tooling will ship in every production Tauri binary until this is fixed. *(30 min)*

### 🟠 Medium (Do This Sprint)

- [ ] **R4 — Eliminate `unwrap()`/`expect()` from `crates/` production paths** — Grep reveals 114+ matches across `oz-api` route handlers and `oz-core` services. Each is a potential panic in a production cashier session. Replace with `?`-propagation or explicit error returns. *(1–2 days)*
- [ ] **R5 — Split `platform/core/src/settings.rs` (95 KB) and `platform/kernel/src/kernel.rs` (64 KB)** — These single-file behemoths need the same treatment as `oz-core`. Extract sub-modules for settings categories and kernel lifecycle phases. *(Half day each)*
- [ ] **R6 — Complete the Thai locale (`*.th.ftl`) bundles** — `purchasing.th.ftl` is 869 bytes vs 1,803 bytes for English. The Thai bundle is missing roughly half of the keys. Run `scripts/translate-stub.py` to generate stubs for all missing keys. *(Half day)*
- [ ] **R7 — Add production test files for `LicenseActivationScreen` and `SessionLockScreen`** — These are high-risk auth paths with zero test coverage. *(Half day)*

### 🟡 Low (Next Sprint)

- [ ] **R8 — Document why 5 feature directories have no `register.ts/tsx`** — `auth`, `restaurant`, `retail`, `setup`, `workspaces` don't self-register. If intentional, document it. If unintentional, add registration files. *(30 min)*
- [ ] **R9 — Upgrade ESLint from 8.57.0 to 9.x** — ESLint 8 is in maintenance mode. *(Half day)*
- [ ] **R10 — Complete P5: 45-page Manual QA Walkthrough** — Still the only item outstanding from the original review. Only 8/45 pages verified after 6+ sprints. *(2 focused days)*

---

## Progress Since Last Review

| Item | Status | Notes |
|------|--------|-------|
| P1 — `oz-core` modularization | 🟡 Half done | Models migrated; DB layer still in `oz-core` |
| P2 — `App.tsx` self-registration | ✅ Done | `registerAllFeatures()` working, 29 features |
| P3 — Sync conflict strategy | ✅ Done | `conflict.rs` fully implements CRDT + status DAG |
| P4 — `rlua` → `mlua` | ✅ Done | Sandboxed, 10 MiB cap, no `os.execute` |
| P5 — Manual QA walkthrough | ❌ Not done | 8/45 pages verified |
| P6 — Repository noise | ✅ Done | `.gitignore` updated; noise files not tracked |
| P7 — React-only decision | ✅ Done | ADR #30 written, footnote removed |
| P8 — Context provider audit | ✅ Done | `AppProviders.tsx` created, pyramid flattened |
| P9 — `ARCHITECTURE.md` updates | ✅ Done | Crate count, module count corrected |
| P10 — `nul` device files | ✅ Done | All `nul` paths ignored |

> **Honest verdict on P1**: The claim that P1 is "complete" is overstated. `oz-core/src/db/sales.rs` at 142 KB proves the most expensive part of the migration (database access decoupling) has not happened. R2 in this review is a direct continuation of P1.

---

## Section 1 — What Is Genuinely Better ✅

### ✅ 1. Sync Conflict Resolution is Production-Quality

`platform/sync/src/conflict.rs` (882 lines) correctly implements:
- **Status DAG** for sales: `active → pending → completed → voided → refunded` — no reversal allowed.
- **CRDT delta-merge** for stock movements: preserves both terminal deltas under concurrent edits.
- **LWW** only for reference data (products, tax rates) where it is semantically correct.

This was the most dangerous architectural gap in the last review. It is now closed.

### ✅ 2. Business Modules Have Real Code

All 10 modules now contain real `services/`, `repositories/`, and `models/` code — not lifecycle stubs. `modules/sales/src/service.rs` correctly orchestrates `process_checkout` with Cart state transitions and DB transactions.

### ✅ 3. CI/CD Pipeline is Enterprise-Grade

7 GitHub Actions workflows:
- **`ci.yml`**: fmt + clippy + nextest sharded across 5 crate groups, sccache/rust-cache.
- **`nightly.yml`**: Full matrix (Linux + Windows + macOS), E2E Docker Compose, benchmark regression.
- **`android.yml` + `ios.yml`**: Signed APK/AAB and IPA builds on tag push.
- **`security.yml`**: Weekly `cargo audit` + `cargo deny` (license and advisory checks).
- **`docs.yml`**: `cargo doc` deployed to GitHub Pages.

### ✅ 4. DevEx Tooling is Exceptional

- **54 scripts** with dual Bash/PowerShell support.
- `graphify` knowledge graph: **23.4 MB, 13,719 nodes, 34,187 edges**, auto-rebuilt on commit.
- Pre-commit hooks: 4 gates (cargo fmt, i18n lint, bundle parity, FTL dedupe).
- `scripts/translate-stub.py` (21 KB): automated locale stub generation.

### ✅ 5. Docker Setup is Production-Ready

Multi-stage `Dockerfile.server` (Rust 1.88 → Debian bookworm-slim), non-root `ozpos` user, `gosu` privilege drop, 4 services with healthchecks and persistent volumes.

### ✅ 6. Lua Scripting is Safely Sandboxed

`oz-lua` integrates `mlua 0.9` with 100K instruction limit, 10 MiB memory cap, disabled `os.execute`/`io`, and defined extension hooks (`apply_discount`, `calc_line_tax`, `validate_order`).

### ✅ 7. Test Suite Breadth

- **203 UI test files** in `ui/src/__tests__/`
- **4 fuzzing targets**: `cart_deser`, `lua_parse`, `money_parse`, `sku_parse`
- All 10 business modules have `#[cfg(test)]` blocks
- Robust `renderWithFluent`/`renderWithProviders` test utilities

---

## Section 2 — The Real Problems 🔴

### 🔴 CRITICAL — Private Signing Key Committed to Repository

`oz-pos-updater.key` (348 bytes, passphrase-encrypted `rsign` key) is in git history. The file is in `.gitignore` but was committed before or around the ignore rule.

**Impact**: Anyone with read access can attempt to decrypt the private signing key. A compromised key means malicious OZ-POS updater binaries can be signed and delivered to real merchants.

**Remediation**:
```bash
# 1. Remove from index and history
git filter-repo --path oz-pos-updater.key --invert-paths
# 2. Generate a new minisign keypair
# 3. Store new private key ONLY in GitHub Secrets
# 4. Update tauri.conf.json pubkey field
```

### 🔴 CRITICAL — `oz-core/src/db/` is Still a Monolith

| File | Size |
|------|------|
| `oz-core/src/db/sales.rs` | **142 KB (3,523 lines)** |
| `oz-core/src/features.rs` | 69 KB |
| `oz-core/src/config_validator.rs` | 17 KB |

Every sales-related change still forces a recompile of `oz-core` and all 29 downstream crates. The stated architecture goal — "Modules Own Business Logic" — is still not true at the database layer.

**What remains:**
1. Move `oz-core/src/db/sales.rs` → `modules/sales/src/repository.rs`
2. Repeat for each domain
3. `oz-core/src/db/` should contain only migration infrastructure and shared query utilities

### 🔴 CRITICAL — `DevToolbar` Ships in Production

```tsx
// App.tsx — no guard
export default function App() {
  return (
    <AppProviders>
      <AppShell />
      <DevToolbar />   {/* ← Ships in every production Tauri binary */}
    </AppProviders>
  );
}
```

**Fix (lazy import):**
```tsx
const DevToolbar = import.meta.env.DEV
  ? lazy(() => import('@/features/design/DevToolbar').then(m => ({ default: m.DevToolbar })))
  : null;
```

### 🟠 MEDIUM — 114+ Panics in Production `crates/`

A grep for `unwrap(` and `expect(` (excluding test blocks) returns 114+ matches concentrated in `oz-api/src/routes/*.rs` and `oz-core/src/audit.rs`. In a live POS cashier session, any of these can crash the Tauri process mid-transaction.

### 🟠 MEDIUM — Platform Layer Files Too Large

| File | Size |
|------|------|
| `platform/core/src/settings.rs` | 95 KB |
| `platform/kernel/src/kernel.rs` | 64 KB |

These need the same sub-module extraction applied to `oz-core`.

### 🟠 MEDIUM — Thai Locale Has ~52% Missing Keys

| File | Size |
|------|------|
| `purchasing.ftl` (English) | 1,803 bytes |
| `purchasing.id.ftl` (Indonesian) | 1,866 bytes |
| `purchasing.th.ftl` (Thai) | **869 bytes** |

Thai-language users see raw Fluent key IDs instead of translated text.

### 🟡 LOW — Auth Screens Missing Tests

`LicenseActivationScreen.tsx` and `SessionLockScreen.tsx` have no test files. These are high-risk auth paths.

### 🟡 LOW — 5 Unregistered Feature Directories

`auth`, `restaurant`, `retail`, `setup`, `workspaces` have no `register.ts/tsx`. No documentation explains why. Ambiguity in a self-registration pattern is a maintenance hazard.

---

## Section 3 — Updated Scorecard

| Dimension | Last Review | This Review | Δ | Notes |
|-----------|-------------|-------------|---|-------|
| Architecture Design | 8/10 | 8/10 | → | Conflict resolution excellent; `oz-core` DB still unfinished |
| Backend Code Quality | 7/10 | 7/10 | → | mlua ✅ conflict.rs ✅ — `oz-core` DB still 142 KB |
| Frontend Code Quality | 6/10 | 7/10 | ↑ | `AppProviders` ✅ self-registration ✅ — DevToolbar ❌ |
| Test Coverage | 8/10 | 8/10 | → | 203 test files, 4 fuzzing targets — 2 auth gaps |
| i18n / Accessibility | 9/10 | 8/10 | ↓ | Thai locale parity gap discovered |
| DevEx / Tooling | 8/10 | 9/10 | ↑ | 7 CI workflows, 54 scripts, graphify active |
| Documentation | 7/10 | 8/10 | ↑ | CHANGELOG 138 KB, 31 ADRs, ARCHITECTURE.md corrected |
| Security Posture | 6/10 | 5/10 | ↓ | Private updater key committed to repository |
| Sync / Offline Strategy | 5/10 | 8/10 | ↑↑ | `conflict.rs` fully implements CRDT + status DAG |
| **Overall** | **7/10** | **7.5/10** | **↑** | Meaningful progress; new critical security gap |

---

## Section 4 — Strategic Assessment

### What Has Changed

The first review found an architecture that existed on paper but not in code. After P1–P10:
- The sync strategy is genuinely production-safe for financial data.
- The UI registry pattern is actually working across 29 features.
- The DevEx infrastructure is world-class for a small team.

### What Has Not Changed

`oz-core` is still carrying the weight. Models moved — but a 3,523-line database file is not decoupled business logic.

### The New Risk: Committed Private Key

This is the most urgent finding. A compromised updater signing key is a supply-chain attack vector against every merchant who installs OZ-POS. Fix R1 before any other work this sprint.

### Recommendation: Stabilisation Sprint

Declare a stabilisation sprint. No new features. In order:
1. **R1** — Remove private key from git history and rotate (2 hours, highest urgency)
2. **R3** — Guard DevToolbar (30 minutes)
3. **R4** — Eliminate `unwrap()` in route handlers (2 days)
4. **R10** — Complete manual QA walkthrough (2 days)

Then resume feature development.

---

*This review was generated by code analysis of the actual source tree across three parallel audits — Rust backend, React/TypeScript frontend, and DevOps/tooling. Every finding is traceable to a specific file and line range.*
