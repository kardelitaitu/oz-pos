# OZ-POS Sprint Roadmap Checklist

> *Solo Developer Execution Roadmap — Incomplete Tasks Breakdown*

---

## 🚀 Sprint 1: Mobile Parity
*Target: Get the POS running on native Android and iPad tablets.*

- [x] Setup Android SDK & environment
- [x] Configure Tauri mobile capabilities
- [x] Build Android APK (`app-universal-release-unsigned.apk` generated)
- [ ] *Note: iOS build is deferred due to Windows OS constraints*

---

## 🌐 Sprint 2: Localization & Accessibility
*Target: Make the app fully accessible and translated, finishing the i18n migration.*

- [x] Audit React UI for hardcoded English strings (100 feature files audited)
- [x] Wrap strings in `<Localized>` component (`StaffManagementScreen`, `TaxConfigurationScreen`, `TerminalManagementScreen`, `StockTransfersScreen`, etc.)
- [x] Sync `en-US.ftl`, `id.ftl`, and Thai translations (`verify-bundle-parity.py` 0 missing keys & `dedupe-ftl.py` clean)
- [x] Run Lighthouse a11y audit & static checks
- [x] Fix ARIA tags and color contrasts

---

## 📊 Sprint 3: Reporting & Diagnostics
*Target: Complete reporting interface & performance profiling tooling.*

- [x] Wire Home Dashboard to real SQLite data via Tauri IPC (`export_daily_summary`, `get_daily_revenue`, etc.)
- [x] Wire "Print Report" button to ESC/POS printer driver (`printSalesReceipt` integration)
- [x] Implement empty states for report screens (`SalesReportScreen`, `InventoryReportScreen`, `SalesDashboardScreen`)
- [x] Add `tokio-console` integration macros (`platform/startup/src/console.rs`)
- [x] Add `cargo flamegraph` helpers (`scripts/flamegraph.ps1` & `scripts/flamegraph.sh`)
- [x] Run Criterion benchmarks (`barcode_lookup`, `transaction_commit`)

---

## 🛒 Sprint 4: Advanced Retail & F&B Features
*Target: Build loyalty, promotions engine, and product bundle capabilities.*

- [ ] Loyalty Program (DB schema, API, UI)
- [ ] Promotions Engine (Lua rules for buy-X-get-Y, % off)
- [ ] Promotions Management UI
- [ ] Product Bundles schema & UI

---

## 🍽️ Sprint 5: Specialized UIs
*Target: Deliver restaurant & kiosk dedicated user interfaces.*

- [ ] KDS (Kitchen Display System) route
- [ ] Self-Service Kiosk route (locked down UI)
- [ ] Table Management UI (floor plan)

---

## 🎨 Sprint 6: Theming & Plugin Ecosystem
*Target: Open-source extensibility, plugin framework, and brand customization.*

- [ ] Theming Engine (logo upload, primary color picker)
- [ ] Plugin Architecture (`plugin.toml`, API)
- [ ] Developer Documentation (`plugin-guide.md`, `quickstart.md`)
