# Changelog

All notable changes to OZ-POS are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- *(none yet)*

## [0.0.3] — 2026-06-30

### Added
- Pre-commit hook (auto `cargo fmt --all`)
- CI fixes for cross-platform compilation (macOS keychain, Linux libudev+zbus, Windows Tauri)

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
