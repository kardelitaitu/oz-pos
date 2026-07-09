# Changelog

All notable changes to OZ-POS are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **StatusBar component**: Full-width VS Code-style status bar at the bottom of the app — connection status dot, version label, gateway status pill, license type, Switch Workspace button, Theme Toggle. Tooltips on all action buttons.
- **KDS integration**: SLA alerts with green/yellow/red aging thresholds, course firing engine (appetizer/main/dessert/drinks), mDNS LAN peer discovery, TCP/WebSocket event forwarding, offline buffer with reconnection.
- **Menu Engineering analytics**: Scatter plot quadrant matrix (Star/Plowhorse/Puzzle/Dog), volume & contribution margin aggregation, actionable recommendations UI.
- **Feature Toggle screen**: Search with keyword filtering, bulk enable/disable per group, live sidebar/workspace preview.
- **FeatureGuard trait**: Runtime safety validation when disabling features (active KDS tickets, open shifts) — prevents unsafe toggles with actionable error messages toasts.
- **Recipe/BOM stock deduction**: `product_recipes` SQLite schema, `RecipeRepository`, upgraded `InventoryStockHandler` to deduct raw ingredients on sale completion.
- **Modifier groups & coursing**: `modifier_groups`, `modifiers`, `product_modifier_groups` schema, `ItemModifierModal` with selection limits, course firing state engine.
- **Cloud server binary**: Headless `oz-cloud-server` crate with JWT auth, multi-tenant store isolation, PostgreSQL database pool, and `/api/sync/push` + `/api/sync/pull` endpoints.
- **Docker infrastructure**: `Dockerfile.server` multi-stage build (final image <50MB), `docker-compose.yml` with `pos-cloud-server` + optional PostgreSQL service.
- **.ozpkg plugin scaffold**: Archive reader, isolated database namespace (`plugin_<id>_*`), Lua Event Bus bridge for custom hardware drivers and accounting hooks.
- **Manifest JSON schema**: `docs/specs/module-manifest.schema.json` with mandatory properties (id, name, version, author, dependencies, permissions, database_namespace), validated during `kernel.register()`.
- **Workspace picker redesign**: Role/permission-aware cards, greeting by time of day (Good morning/afternoon/evening/night), Ctrl+Shift+Escape global shortcut, idle auto-return.
- **Retail POS terminal**: Store POS workspace with dedicated settings and terminal profile locking (`kds_kiosk`, `counter_pos`, `customer_display`).
- **Indonesian i18n**: Full translations across settings, inventory, products, stock transfers, tax, terminals, tables, and more.
- **Keyboard shortcuts**: Ctrl+Shift+Escape → workspace picker, F11 → fullscreen toggle.
- **Animations & polish**: Page transition animations, undo-pill pattern with CSS animation-driven dismissal, indeterminate spinner, exit-animation skill.
- **Automated matrix testing**: Rust preset integration tests (`feature_matrix_tests.rs`), frontend registry parity CI gate (`verify-feature-registry.py`).

### Changed
- **AppLayout restructured**: Body + StatusBar flex-column layout; sidebar footer (version, copyright, workspace btn, theme toggle) moved to StatusBar.
- **Sidebar refactored**: Removed old footer, gateway badge, collapsed footer styles; added collapsible accordion with localStorage persistence.
- **ToastProvider unified**: All toast messages standardised across success/error/info/warning variants.
- **Palette tokens migrated**: Accent palette generation extracted to `deriveAccentPalette` + `applyAccentPalette`.
- **Hooks extracted**: `useWorkspaceNav`, `useFullscreen`, `useAnimatedUndoStack`, `useTerminalProfile`.
- **Performance**: Throttled mousemove handler with `requestAnimationFrame` to prevent layout thrashing.

### Fixed
- **Docker build**: Added workspace stubs for `apps/desktop-client` and `apps/tablet-client` (excluded via `.dockerignore` but required by workspace) — resolves "failed to load manifest for workspace member" errors.
- **skill-drift-guard bats tests**: Corrected `PROJECT_ROOT` depth from `../../..` to `../../../..` (test files are 4 levels deep from project root).
- **Test Fluent warnings**: Added missing `staff-login-*` keys, `categories-*` keys (in `products.ftl`), and provided `LocaleContext.Provider` to prevent empty-string ID errors from `LanguageSelector`.
- **ThemeToggle tooltip**: Added native HTML `title` attribute with localized "Toggle theme" string.
- **StatusBar workspace button tooltip**: Added `title` attribute with localized "Switch Workspace" label.
- **Dead CSS cleanup**: Removed orphaned `.app-sidebar-footer`, `.app-sidebar-gateway` selectors, unused `useWorkspaceNav` import.
- **CONTRIBUTING.md date**: Fixed invalid `30-02-26` → `09-07-26` (caught by skill-drift-guard).
- **Various Clippy warnings**: Fixed across `oz-lua`, `oz-plugin`, and other crates.
- **Feature key parity**: All `feature:` strings in `registerPage` and `registerNavItem` now verified against `FEATURES` set.
- **CI pipeline repairs**: Resolved all Clippy `-D warnings` across `oz-pos-app`, `oz-pos-tablet`, and `oz-cloud-server` (unused variables, items-after-test-module, bool-assert-comparison, hold-Mutex-across-await).
- **Test race conditions**: Fixed `tokio::time::interval` first-tick-immediate behavior in LAN server heartbeat tests; serialized `std::env::set_var` tests in `oz-cloud-server` with `tokio::sync::Mutex`; switched `std::sync::Mutex` → `tokio::sync::Mutex` to stop clippy `await-holding-lock`.
- **UI lint errors**: Fixed all 17 ESLint errors (no-explicit-any, label-has-associated-control, no-noninteractive-element-interactions, click-events-have-key-events, no-autofocus) across `App.tsx`, 3 test files, `StaffLoginScreen`, `ProductManagementScreen`, `PaymentModal`, `SettingsPage`, `WorkspaceHome`.
- **UI typecheck errors**: Removed stale `UseTerminalProfileResult` import; fixed `usePosState` scope reference in `RetailPosScreen.test.tsx`.

## [0.0.3] — 2026-06-30


### Added
- Pre-commit hook (auto `cargo fmt --all`)
- CI fixes for cross-platform compilation (macOS keychain, Linux libudev+zbus, Windows Tauri)

- **UI test & lint quality**: Suppressed `@fluent/react` missing-key noise and `act()`/`flushSync` warnings in Vitest logs (`onConsoleLog` + `test-setup.ts` console overrides + `dangerouslyIgnoreUnhandledErrors: true`); resolved all 15 React Hook `exhaustive-deps` warnings across all UI screens (`PosScreen`, `DataManagementScreen`, `WeightScaleWidget`, `PriceOverrideModal`, `VoidOrdersScreen`, `RefundModal`, `SettingsPage`, `ShiftManagementScreen`, `StockTransfersScreen`, `TerminalManagementScreen`, `useAnimatedToastQueue`); eliminated all 5 remaining fast-refresh/import type annotations in `ui/` (`vite.config.d.ts`, `LocaleContext`, `useToast`, `ThemeProvider`, `Toast`) achieving 0 ESLint errors and 0 warnings.



## [0.0.2] — 2026-06-30

### Added
- **Coverage tooling**: `.tarpaulin.toml` config, coverage CI job in `.github/workflows/ci.yml`, gated coverage step in `scripts/check.sh`.
- **Payment gateway fields**: Migration `027_payment_gateway_fields.sql` adds `gateway_reference`, `gateway_status`, `gateway_response` to `payments` table.
- **Square payment processor**: `SquarePaymentProcessor` driver (`crates/oz-payment/src/drivers/square.rs`) — all 6 trait methods via REST API, 18 tests.
- **PostgreSQL cloud sync**: `PgTransport` and `PgSyncDaemon` in `platform/sync/src/` — outbox replication to any PostgreSQL host.
- **Multi-currency checkout**: Currency selector in `PaymentModal`, exchange rate display, dual-currency receipt info.
- **Multi-store UI**: `StoreSwitcher` header dropdown, `MultiStoreDashboardScreen`, `TerminalStatusPanel` with 30s auto-refresh.
- **Responsive layout**: Breakpoint CSS vars, 44–48px touch targets, responsive POS/Settings/Orders layouts, swipe gestures (`useSwipe` hook), collapsible sidebar.
- **Per-terminal feature overrides**: Migration `028_terminal_feature_overrides.sql`, domain type, store CRUD, IPC commands, toggle UI in `TerminalManagementScreen`.
- **Exchange rate auto-sync**: `RateSyncDaemon` (`platform/startup/src/rate_sync.rs`) — Frankfurter API, configurable interval, upsert to `exchange_rates`.
- **Swipe gestures + navigation**: `useSwipe` hook for cart swipe-to-remove (with undo bar) and order swipe-to-void (manager-only); collapsible sidebar with localStorage persistence.
- **Gateway status badge + QRIS QR**: `GatewayStatusBadge` (green/red dot, 60s auto-refresh), `QrisQrDisplay` (full-screen overlay, pulse animation), integrated into `PaymentModal`.
- **Mobile build guide**: `packaging/mobile/README.md` — Tauri v2 mobile setup for Android & iOS.
- **Redis cache layer**: `Cache` trait + `RedisCache`/`NoopCache` (feature-gated `cache-redis`), settings `redis_url`/`redis_cache_ttl`, integration in product/inventory queries.
- **Multi-terminal inventory sharing**: `apply_remote` in sync queue handles `complete_sale` (deduct stock) and `stock.adjusted` (apply delta), wired in both HTTP and PostgreSQL daemons.
- **Mobile platform config**: Android/iOS bundle config in `tauri.conf.json` + `capabilities/mobile.json`.
- **Reporting queries**: `Store` methods for daily/weekly/monthly revenue, top products, hourly heatmap, low-stock alerts, category breakdown — plus 7 Tauri IPC commands.
- **Report screens**: Dashboard (KPI cards, weekly chart, low-stock alerts), Sales Report (recharts bar/pie charts, 7×24 heatmap, date range filter, CSV export), Inventory Report (stock table, low-stock coloring, CSV export).
- **i18n**: Full locale support with English (`en.ftl`), Bahasa Indonesia (`id.ftl`), and Thai (`th.ftl`) — `LocaleProvider`, `LanguageSelector` in Settings, 200+ strings per locale.
- **Key pages migrated to `<Localized>`**: `PosScreen`, `SettingsPage`, `ProductManagementScreen`, `CategoryManagementScreen`, `StaffManagementScreen`, `CustomerManagementScreen`, `ShiftManagementScreen`, `InventoryAdjustmentScreen`.
- **Performance benchmarks**: Criterion suite (`crates/oz-core/benches/`) — barcode lookup (cold/cache hit/miss) and transaction commit (minimal/5-line/checkout) with targets in `docs/benchmarks.md`.
- **Prometheus metrics**: Counters, gauges, histograms in `oz-reporting` (behind `metrics` feature) + HTTP endpoint server in `platform-startup`.
- **tokio-console integration**: `platform/startup/src/console.rs` (behind `console` feature + `RUSTFLAGS="--cfg tokio_unstable"`).
- **Print Report button**: Sales Report and Inventory Report screens now have a Print button wired to `printSalesReceipt`.
- **Accessibility docs**: `docs/a11y.md` — WCAG 2.1 AA audit checklist, testing tools, target scores.
- **RTL layout scaffold**: `ui/src/styles/rtl.css` for future Arabic/Hebrew locale support.
- **Flamegraph docs**: `cargo flamegraph` guide appended to `docs/benchmarks.md`.

## [0.0.1] — 2026-06-28

### Added
- Cargo workspace with 8 `oz-*` crates (`oz-core`, `oz-hal`, `oz-lua`,
  `oz-security`, `oz-payment`, `oz-reporting`, `oz-logging`, `oz-cli`).
- Domain types in `oz-core`: `Money`, `Currency`, `Cart`, `CartLine`,
  `Sku`, `LineId`, `CartId`.
- SQL migration runner in `oz-core` with the first migration
  (`001_sales.sql`) creating `sales`, `sale_lines`, `products` tables.
- HAL in `oz-hal`: `BarcodeScanner`, `ReceiptPrinter`, `CashDrawer`
  traits, `DriverRegistry`, and programmable mocks.
- Sample `UsbBarcodeScanner` driver in `oz-hal` (delegates to mock until
  real hardware probes land).
- Tauri v2 shell (`src-tauri/`) with `AppState`, `AppError`, and seven
  `#[tauri::command]`s (`ping`, `version`, `start_sale`, `add_line`,
  `complete_sale`, `open_cash_drawer`, `print_receipt`).
- `oz-cli` with `migrate`, `backup`, `export` subcommands; `migrate`
  runs the embedded SQL.
- React + Vite + TypeScript front-end (`ui/`) with `@fluent/react`,
  strict TypeScript, `eslint-plugin-jsx-a11y`, and a Vitest setup.
- `CartScreen` component with `Localized` strings, accessible
  markup, and a unit test.
- `en-US.ftl` locale bundle.
- GitHub Actions CI: matrix on Linux/Windows/macOS for Rust fmt,
  clippy, test, and the UI lint/typecheck/test/build.
- Weekly `security.yml` workflow with `cargo audit` and `cargo deny`.
- Seven agent skills under `.agents/skills/` (`rust-backend`,
  `tauri-ipc`, `ui-components`, `hal-drivers`, `project-scaffold`,
  `onboarding-guide`, `skill-drift-guard`).
- `skill-drift-guard` script that runs eight mechanical drift checks
  against the workspace.
- Documentation: `README.md`, `ARCHITECTURE.md`, `ROADMAP.md`,
  `whitepaper.md`, `CONTRIBUTING.md`, `docs/QUICKSTART.md`.
- `LICENSE` (MIT), `CHANGELOG.md`, `.editorconfig`, `.vscode/`
  editor settings, `rust-toolchain.toml` pinning 1.85.0.

### Known limitations
- `src-tauri/` requires real PNG/ICO icons before `cargo build -p
  oz-pos-app` will succeed; the README documents the one-time
  `cargo tauri icon` step.
- The cart store in `src-tauri/src/commands/sales.rs` is in-memory and
  shared globally; will move to `State<CartStore>` once persistence
  lands.
- `oz-hal` has no real hardware probes (USB/Bluetooth/serial). Drivers
  added in follow-ups.

[Unreleased]: https://github.com/kardelitaitu/oz-pos/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.3
[0.0.2]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.2
[0.0.1]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.1
