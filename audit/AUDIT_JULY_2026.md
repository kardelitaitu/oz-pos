# Audit Plan — July 2026

> Date: 2026-07-31
> Status: PLANNING · 36 sectors identified · 0 audited
>
> **Universal Sector Audit Checklist:** Every sector report must evaluate and document:
> - **Functionality & Logic:** Feature correctness, state management, offline resilience, and edge cases.
> - **UI/UX & States:** Consistent loading, error, and empty states, with responsive and touch-friendly interactions.
> - **Accessibility & i18n:** Complete ARIA attributes, keyboard/focus management, and localized strings without hardcoded user-facing text.
> - **Theming:** Strict design-token usage, dark-mode compatibility, and reliable color fallbacks.
> - **Performance:** React render efficiency, Rust/Tauri IPC optimization, query efficiency, and asset/bundle footprint.
> - **Security & Integrity:** Input validation, authorization/role gating, SQL/XSS prevention, and data synchronization boundaries.
> - **Quality Assurance:** Unit/E2E/Rust test coverage, API/code documentation accuracy, and flaky or untested path resolution.
>
> **Convention:** Each sector gets its own detailed report in this folder,
> named `NN-sector-slug.md` (e.g. `01-crm-module.md`). The report includes
> scope, evidence, findings, severity (P0–P4), affected files, recommended
> fixes, validation results, and fix status.

## Sectors to Audit

1. CRM module — CustomerManagementScreen, customer search, customer history
2. Loyalty module — LoyaltyScreen, points, rewards, tier configuration
3. Reporting module — ReportingScreen, report generation, export, filters
4. Currency module — CurrencyScreen, exchange rates, currency formatting
5. Tax module — TaxScreen, tax rules, tax categories, tax calculation — ✅ **FULLY REMEDIATED** (all five phases; see [05-tax-module.md](05-tax-module.md) — residuals documented under [Residual follow-ups](05-tax-module.md#residual-follow-ups-documented-not-blocking))
6. Staff module — StaffManagementScreen, roles, permissions, shifts
7. Inventory module — InventoryScreen, StockInquiry, stock adjustments, transfers — ✅ **FULLY REMEDIATED** (all 11 findings INV-01→INV-11; see [07-inventory-module.md](07-inventory-module.md) — commits `a2c70848`, `45d65511`, `5be6de69`, `3bbd44e9`, `3caddf6e`)
8. Plugin system — Lua plugin engine, example-discount, plugin sandbox — ✅ **FULLY REMEDIATED** (all 11 findings PLG-01→PLG-11; see [08-plugin-system.md](08-plugin-system.md) — commits `64b0281a`, `b9a7fa76`, `da8ea51c`, `95da123e`, `47f63d52`, `06f7ff34`, `308bc101`, plus residuals `4022bc5d`, `7d5d318c`, `cb6d181c`)
9. Sync module — platform/sync, real-time sync, conflict resolution — ✅ **FULLY REMEDIATED** (all 12 findings SYNC-01→SYNC-12; see [09-sync-module.md](09-sync-module.md) — commits `a1ea01e7`, `b722740f`, `5229e296`, `85e323c7`, `5633e790`, `178abfbf`)
10. ProductManagementScreen — ~641 lines, flagged hardcoded aria-labels — ✅ **FULLY REMEDIATED** (all 12 findings PROD-01→PROD-12; see [10-product-management-screen.md](10-product-management-screen.md) — commits `f399c703`, `beba8dad`, `6a6840aa`, `6b9aead9`, `67bb09c1`)
11. CategoryManagementScreen — ~647 lines, flagged hardcoded aria-labels — ✅ **FULLY REMEDIATED** (all 10 findings CAT-01→CAT-10; see [11-category-management-screen.md](11-category-management-screen.md) — commits `3201dcd6`, `9ef25a9f`, `ba6da1f7`, `2e1d28ca`, `382d2e2f`)
12. CustomerManagementScreen — ~429 lines, flagged hardcoded aria-labels — ✅ **FULLY REMEDIATED** (all 11 findings CUST-01→CUST-11; see [12-customer-management-screen.md](12-customer-management-screen.md) — commits `e85137dc`, `973a2dd7`, `a520d170`, `24ea4ad5`, `ec8e39be`, `afc0e290`, `ddc82de8`, `95b54059`, `8bd25e52`)
13. AuditLogScreen — audit trails, log filtering, export — ✅ **FULLY REMEDIATED** (all 11 findings AUD-01→AUD-11; see [13-audit-log-screen.md](13-audit-log-screen.md) — commits `1a4fd1b5`, `174d839f`, `359ad440`, `69abc5af`, `166aa991`, `6e488510`)
14. LocationPicker / LocationManagement — location CRUD, picker UX
15. TableManagement — Restaurant table layouts, drag-and-drop, floor plans
16. Accessibility — full-app ARIA deep dive, screen-reader flow, focus management — ✅ **FULLY REMEDIATED** (all 12 findings A11Y-01→A11Y-12; see [16-accessibility.md](16-accessibility.md) — commits `ef370c19`, `ee8c6580`, `7dd33263`, `00c99b75`, `6c1747a9`, `d8db28c6`, `5c49c449`, `962a0c0f`)
17. Performance — bundle size, render optimization, lazy loading, code splitting — ✅ **FULLY REMEDIATED** (all 10 findings PERF-01→PERF-10; see [17-performance.md](17-performance.md) — commits `2b762b08`, `2e1f3d31`, `50b50836`, `bf376234`, `df753501`)
18. Error handling — error boundaries, toast consistency, retry patterns, fallback UI — ✅ **FULLY REMEDIATED** (all 10 findings ERR-01→ERR-10; see [18-error-handling.md](18-error-handling.md) — commits `10f1bae0`, `c586c3d6`, `537f5867`, `31adb7c3`, `5dacd75f`)
19. Offline resilience — retail POS offline mode, queueing, sync-on-reconnect — ✅ **FULLY REMEDIATED** (all 12 findings OFF-01→OFF-12; see [19-offline-resilience.md](19-offline-resilience.md) — commits `91766573`, `233eed6b`, `17ee223c`, `e07ec4ae`)
20. Mobile / tablet responsiveness — index.tablet.html, touch UX, viewport — ✅ **FULLY REMEDIATED** (all 6 findings TAB-01→TAB-06; see [20-tablet-responsiveness.md](20-tablet-responsiveness.md) — commits `42263ef9`, `ed6ec31f`, `7780c206`, `6aedc287`, `27a1e0e1`, `7a82227a`)
21. Theme system — token completeness, dark mode gaps, color-mix fallbacks — ✅ **FULLY REMEDIATED** (all 6 findings THM-01→THM-06; see [21-theme-system.md](21-theme-system.md) — commits `2ca5e5a8`, `57b23bd4`, `b5fa60a5`, `cb9544c1`, `4a150495`)
22. Keyboard shortcuts — coverage audit across all screens — ✅ **FULLY REMEDIATED** (all 10 findings KEY-01→KEY-10; see [22-keyboard-shortcuts.md](22-keyboard-shortcuts.md) — commits `7a5e7cdd`, `2f981b8e`, `544ea5cf`, `92832424`, `db3e18d8`, `e233beae`)
23. Loading states — skeleton consistency, loading spinners, progress indicators — ✅ **FULLY REMEDIATED** (all 10 findings LOAD-01→LOAD-10; see [23-loading-states.md](23-loading-states.md) — commits `6d1a21ca`, `710cca22`, `3dd82a3f`, `9de773c1`, `8488ba05`, `13bfdf40`)
24. Empty states — consistency across all data views and lists
25. Rust backend — clippy warnings, unsafe blocks, error propagation, API coherence
26. Docker images — size optimization, layer caching, security scanning
27. CI pipeline — gate completeness, flaky test quarantine, cache efficiency
28. Release process — versioning, changelog automation, artifact signing
29. Database migrations — idempotency, rollback coverage, index coverage, schema docs
30. API documentation — api-reference.md drift vs actual Tauri commands
31. Test coverage — files below 50%, untested edge cases, missing integration tests
32. Flaky tests — quarantine, retry policy, flake root-cause tracking
33. E2E spec coverage — critical-path gaps, cross-module flows, error paths
34. Fuzz / property tests — Rust fuzz targets, proptest coverage
35. Input validation — form sanitization, SQL injection surface, XSS vectors
36. Auth / authorization — role gating completeness, session timeout, token hygiene
