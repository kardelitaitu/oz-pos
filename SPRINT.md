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

- [/] Audit React UI for hardcoded English strings
- [ ] Wrap strings in `<Localized>` component (`StaffManagementScreen`, `TaxConfigurationScreen`, `TerminalManagementScreen`, `StockTransfersScreen`, etc.)
- [ ] Sync `en-US.ftl`, `id.ftl`, and Thai translations (`verify-bundle-parity.py` & `dedupe-ftl.py`)
- [ ] Run Lighthouse a11y audit
- [ ] Fix ARIA tags and color contrasts

---

## 📊 Sprint 3: Reporting & Diagnostics
*Target: Complete reporting interface & performance profiling tooling.*

- [ ] Wire Home Dashboard to real SQLite data via Tauri IPC
- [ ] Wire "Print Report" button to ESC/POS printer driver
- [ ] Implement empty states for report screens
- [ ] Add `tokio-console` integration macros
- [ ] Add `cargo flamegraph` helpers
- [ ] Run Criterion benchmarks

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
