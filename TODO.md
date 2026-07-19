# 0.0.13 — Plugin Hardening + Sync Reliability + Performance

> **Goal:** Harden the Lua plugin sandbox, improve offline-sync conflict resolution, profile and optimize UI rendering, and close remaining documentation/ADR gaps.

**Current state:** 0 / ~25 items · Updated 2026-07-19

---

## Progress Summary

| Area | Total | Done | Progress |
|------|-------|------|----------|
| 🔴 P0 — Plugin Security | 5 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟢 P1 — Sync Reliability | 6 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟡 P2 — UI Performance | 5 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🔵 P3 — KDS Enhancements | 5 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| 🟣 P4 — Docs & Compliance | 4 | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |
| **Total** | **25** | **0** | **▱▱▱▱▱▱▱▱▱▱ 0%** |

---

## 🔴 P0 — Plugin Security (Lua Sandboxing)

**Goal:** Audit and harden the Lua plugin execution environment to prevent privilege escalation, data leaks, and DoS from malicious or buggy plugins.

### Background

The plugin system (`crates/oz-lua/`) allows Lua scripts to intercept sale events, modify cart totals, and trigger stock adjustments via `oz-plugin` and `oz-lua`. Currently:
- Plugins run in a standard `mlua` Lua VM with **no sandbox restrictions**
- `require` is unrestricted — plugins can load any LuaRocks module
- No CPU instruction limit is set
- No memory/heap limit is configured
- No filesystem access restriction (no `chroot` or seccomp)
- No network access restriction

### Checklist

- [ ] **P0-1: Sandbox audit** — Review all exported `oz-plugin` functions and determine minimum needed permissions per plugin type (discount, tax, validation, reporting)
- [ ] **P0-2: Implement permission manifests** — Add `required_permissions` field to `plugin.toml` manifest (e.g., `["cart:read", "cart:write", "inventory:read"]`). Reject plugins with undeclared permissions at load time.
- [ ] **P0-3: Resource limits** — Set `mlua` instruction limit (`set_instruction_limit(100_000)`), memory limit via Lua `collectgarbage` hooks, and execution timeout via tokio select! with 5-second deadline.
- [ ] **P0-4: Safe environment** — Stub out dangerous globals (`os.execute`, `io.open`, `loadfile`, `dofile`, `require`). Provide whitelisted `oz.*` API subset. Test that malicious scripts can't escape.
- [ ] **P0-5: Regressions** — Verify existing example plugins (`plugins/example-discount/discount.lua`, `scripts/examples/discount_bulk.lua`, `tax_overrides.lua`, `validate_order.lua`) still work with the sandboxed environment.

---

## 🟢 P1 — Offline-Sync Reliability

**Goal:** Improve conflict resolution during multi-terminal offline sync, add comprehensive integration tests, and harden error recovery paths.

### Background

The sync system (`platform/sync/`) uses cursor-based push/pull with exponential backoff. Current known gaps:
- No conflict resolution strategy for concurrent edits to same product/sale from different terminals
- No integration tests for the full sync lifecycle (enqueue → push → pull → apply)
- Batch splitting works but edge cases around auth expiry mid-batch are untested
- Snapshot import recovery path is untested

### Checklist

- [ ] **P1-1: Conflict resolution strategy** — Define and implement a last-writer-wins (LWW) strategy using `updated_at` timestamps for reference data (products, categories, tax rates) and CRDT-merge for sales and stock movements. Document in ADR-21.
- [ ] **P1-2: Sync integration tests** — Add integration tests covering: full push→pull lifecycle, auth expiry mid-batch retry, concurrent edits from two terminals (LWW resolution), partial batch failure recovery, and snapshot import after anchor expiry.
- [ ] **P1-3: Conflict UI indicators** — Add visual indicators in the UI when sync conflicts are detected: warning badge on OfflineQueueScreen, conflict count in StatusBar, and a "Resolve Conflicts" sub-screen showing conflicted items with resolution options.
- [ ] **P1-4: Snapshot import error handling** — Test `import_snapshot` with corrupted/malformed snapshots, partial imports (abort mid-way), and concurrent snapshot imports during active sync. Add idempotency guards.
- [ ] **P1-5: Offline queue dedup hardening** — Verify `enqueue` deduplication by `sale_id` works correctly when the same sale is enqueued multiple times across different terminals. Add tests.
- [ ] **P1-6: Sync observability** — Add per-terminal sync status to the dashboard, including: last sync time, pending item count, failed item count, average sync duration, and conflict count. Expose via new Tauri command + settings screen.

---

## 🟡 P2 — UI Performance Optimization

**Goal:** Profile and optimize the three most expensive renders: product lookup grid, KDS ticket board, and sales history modal.

### Background

Current UI test suite runs in ~19s. The product grid (ProductLookupScreen/RetailPosScreen) re-renders all items on every keystroke in the search bar. The KDS ticket board polls every 5 seconds. Sales history modals re-query the full sale on every open.

### Checklist

- [ ] **P2-1: Profile baseline** — Add React Profiler traces to ProductLookupScreen, KDS ticket board, and SalesHistoryScreen. Record baseline render times and re-render counts in CI test output.
- [ ] **P2-2: Product grid virtualization** — Replace flat product list with `react-window` virtualized grid (FixedSizeGrid for desktop, FixedSizeList for tablet). Render only visible products + 2 rows overscan. Expected: 40%+ reduction in initial render time.
- [ ] **P2-3: KDS polling backoff** — Replace fixed 5-second polling with adaptive interval: poll every 2s when tickets are new/unread, back off to 10s when idle for >30s, back off to 30s when idle for >2min. Add `document.visibilityState` check to pause polling when tab is hidden.
- [ ] **P2-4: Sale detail caching** — Cache sale details in a `Map<saleId, Sale>` after first fetch in SalesHistoryScreen. Invalidate on any status change (void, refund, complete). Avoid re-fetching the same sale on modal re-open.
- [ ] **P2-5: Memo audit** — Add `React.memo` to the top 10 most-rendered components identified by profiling: ProductCard, CartLineItem, KDSTicket, KDSTicketTimer, PaymentMethodCard, SearchResultItem, TransactionRow, ShiftSummaryCard, AlertItem, LocationOption. Verify with before/after render counts.

---

## 🔵 P3 — KDS Display Enhancements

**Goal:** Improve KDS screen usability with overdue escalation, sound alerts, and layout polish.

### Background

The KDS system (kitchen display) has multi-layout support (Focus/Kanban/Metro) but lacks overdue escalation (tickets don't visually escalate as they get older), sound alerts for new tickets, and layout parameter persistence.

### Checklist

- [ ] **P3-1: Overdue escalation** — Add progressive visual escalation for tickets: 5min overdue → amber border + pulse, 10min overdue → red border + shake animation, 15min overdue → red background + urgent badge. Use CSS `@keyframes` with `animation-delay` tied to elapsed time.
- [ ] **P3-2: Sound alerts** — Add optional sound notification when a new ticket arrives: short chime via `AudioContext` oscillator (no external audio file needed). One sound per ticket, debounced to max 1 sound per 5 seconds. Toggle in KDS settings.
- [ ] **P3-3: Layout persistence** — Save selected KDS layout (Focus/Kanban/Metro) per terminal to localStorage. Restore on reload. Add `lastLayout` to `KdsLayoutSwitcher` state.
- [ ] **P3-4: Ticket count badge animation** — Animate ticket count changes on column headers with a brief scale-up bounce (0.3s) when count increases, scale-down when count decreases. CSS-only via `@keyframes`.
- [ ] **P3-5: KDS settings panel** — Add a settings gear icon in KDS header that opens a popover with: sound toggle, overdue escalation time thresholds (slider: 3-15min), auto-acknowledge new tickets toggle, and display density (comfortable/compact).

---

## 🟣 P4 — Documentation & Compliance

**Goal:** Close remaining doc gaps: ADR status updates, missing `///` docs, skill-drift audit, and changelog completeness.

### Background

Several ADRs lack final "Implemented" status updates. The skill-drift-guard found minor drift. Some recently added modules lack full doc comments.

### Checklist

- [ ] **P4-1: ADR status audit** — Review all ADRs in `docs/decisions/`. Update ADR-18 (Multi-Location Inventory), ADR-19 (Sale Deduction), ADR-20 (Payment-Capture) from draft → Implemented with completion dates. Verify all 6 ADR-20 acceptance criteria are covered by passing tests.
- [ ] **P4-2: Missing docs** — Audit recently added modules for missing `///` docs: `crates/oz-core/src/location_resolver.rs`, `crates/oz-core/src/sale_deduction.rs`, `crates/oz-core/src/cache.rs` (RedisCache methods). Run `cargo clippy -- -D warnings` to verify zero missing-docs warnings.
- [ ] **P4-3: Skill-drift guard** — Run `.agents/skills/skill-drift-guard/scripts/detect.sh --report` and fix any findings. Ensure all installed skills reference valid paths, crates, and types.
- [ ] **P4-4: CHANGELOG final pass** — Verify CHANGELOG.md has entries for all versions (0.0.12, 0.0.13). Cross-reference git log to ensure no commits are undocumented. Add any missing entries.

---

## 🧭 Dependency Graph

```
🔴 P0 Plugin Security ───── independent (no deps)

🟢 P1 Sync Reliability
    ├── P1-1 Conflict strategy (ADR-21 draft)
    ├── P1-2 Integration tests (depends on P1-1)
    ├── P1-3 Conflict UI (depends on P1-1)
    ├── P1-4 Snapshot hardening (independent)
    ├── P1-5 Dedup tests (independent)
    └── P1-6 Observability (independent)

🟡 P2 UI Performance
    ├── P2-1 Profile baseline ──────────────┐
    ├── P2-2 Product grid virtualization ────┤
    ├── P2-3 KDS polling backoff ───────────┤── all independent
    ├── P2-4 Sale detail caching ───────────┤
    └── P2-5 Memo audit ────────────────────┘

🔵 P3 KDS Enhancements ─ all independent

🟣 P4 Docs & Compliance ─ all independent
```

---

## 🎯 Estimated Effort

| Priority | Item | Est. Effort | Dependencies |
|----------|------|-------------|--------------|
| 🔴 | P0-1: Sandbox audit | 1 hr | None |
| 🔴 | P0-2: Permission manifests | 2–3 hrs | P0-1 |
| 🔴 | P0-3: Resource limits | 1–2 hrs | P0-1 |
| 🔴 | P0-4: Safe environment | 2–3 hrs | P0-1 |
| 🔴 | P0-5: Plugin regressions | 1 hr | P0-2, P0-3, P0-4 |
| 🟢 | P1-1: Conflict strategy | 3–4 hrs | None (ADR-21) |
| 🟢 | P1-2: Sync integration tests | 3–4 hrs | P1-1 |
| 🟢 | P1-3: Conflict UI | 2–3 hrs | P1-1 |
| 🟢 | P1-4: Snapshot hardening | 1–2 hrs | None |
| 🟢 | P1-5: Dedup hardening | 1 hr | None |
| 🟢 | P1-6: Sync observability | 2–3 hrs | None |
| 🟡 | P2-1: Profile baseline | 1 hr | None |
| 🟡 | P2-2: Grid virtualization | 3–4 hrs | P2-1 |
| 🟡 | P2-3: KDS polling backoff | 1–2 hrs | None |
| 🟡 | P2-4: Sale detail caching | 1–2 hrs | None |
| 🟡 | P2-5: Memo audit | 1–2 hrs | P2-1 |
| 🔵 | P3-1: Overdue escalation | 1–2 hrs | None |
| 🔵 | P3-2: Sound alerts | 1–2 hrs | None |
| 🔵 | P3-3: Layout persistence | 1 hr | None |
| 🔵 | P3-4: Ticket count animation | 1 hr | None |
| 🔵 | P3-5: KDS settings panel | 2–3 hrs | None |
| 🟣 | P4-1: ADR status audit | 1 hr | None |
| 🟣 | P4-2: Missing docs | 1 hr | None |
| 🟣 | P4-3: Skill-drift guard | 30 min | None |
| 🟣 | P4-4: CHANGELOG final pass | 30 min | None |

**Total estimated effort:** ~35–45 hours

### Suggested sprint plan

| Sprint | Items | Est. hours |
|--------|-------|------------|
| **Day 1** | P0-1, P0-2, P0-3 (sandbox audit + manifests + limits) | 5–7h |
| **Day 2** | P0-4, P0-5 (safe env + regressions), P4-1, P4-2, P4-3, P4-4 (docs) | 6–9h |
| **Day 3** | P1-1, P2-1 (conflict strategy + profile baseline) | 4–5h |
| **Day 4** | P1-2, P1-3 (sync tests + conflict UI), P2-2 (grid virtualize) | 8–10h |
| **Day 5** | P1-4, P1-5, P1-6 (sync remaining), P2-3, P2-4, P2-5 (perf remaining) | 6–8h |
| **Day 6** | P3-1 through P3-5 (KDS full pass) | 6–9h |
