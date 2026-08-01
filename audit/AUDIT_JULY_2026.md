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
8. Plugin system — Lua plugin engine, example-discount, plugin sandbox
9. Sync module — platform/sync, real-time sync, conflict resolution
10. ProductManagementScreen — ~641 lines, flagged hardcoded aria-labels
11. CategoryManagementScreen — ~647 lines, flagged hardcoded aria-labels
12. CustomerManagementScreen — ~429 lines, flagged hardcoded aria-labels
13. AuditLogScreen — audit trails, log filtering, export
14. LocationPicker / LocationManagement — location CRUD, picker UX
15. TableManagement — Restaurant table layouts, drag-and-drop, floor plans
16. Accessibility — full-app ARIA deep dive, screen-reader flow, focus management
17. Performance — bundle size, render optimization, lazy loading, code splitting
18. Error handling — error boundaries, toast consistency, retry patterns, fallback UI
19. Offline resilience — retail POS offline mode, queueing, sync-on-reconnect
20. Mobile / tablet responsiveness — index.tablet.html, touch UX, viewport
21. Theme system — token completeness, dark mode gaps, color-mix fallbacks
22. Keyboard shortcuts — coverage audit across all screens
23. Loading states — skeleton consistency, loading spinners, progress indicators
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
