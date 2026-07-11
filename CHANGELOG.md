# Changelog

All notable changes to OZ-POS are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [0.0.5] — 2026-07-11

### Added
- **Store-first tenancy (ADR #4)**: Workspace type/instance separation with `SessionContext`, `StoreDatabaseManager` for per-store SQLite files, device-bound auto-boot (`device_bindings` table, HMAC signing), boot resolution engine, and store switcher integration with workspace re-resolution. Tablet shell redesigned with device-bound auto-boot and dynamic workspace tabs.
- **Session token infrastructure (ADR #4)**: `create_session`, `destroy_session`, `resolve_session` commands; frontend session token integration (create/destroy on workspace selection + store switch); `verify-no-raw-params.sh` CI enforcement script integrated into `check.sh`.
- **Subscription tier entitlement (ADR #5)**: Tier infrastructure with quota enforcement, `InstanceStatus` enum (Active/Suspended/Expired), bootstrap free tier; entitlement checks during workspace listing that filter by subscription tier allowed types; clock rollback detection, 14-day offline grace period, effective tier enforcement; auto-recovery on upgrade, safe suspension on downgrade, transaction-safe status transitions with `last_accessed_at` tracking.
- **CRDT delta ledger for inventory (ADR #6)**: `stock_movements` table with CRDT delta ledger pattern; `adjust_stock_with_reason` and `get_stock_from_ledger` commands; `rebuild_stock_summary()` to recompute stock from delta ledger (sync-ready); source terminal/user audit fields populated from session context; version optimistic concurrency on products and sales tables (`version` column, wired into `update_product`, `update_sale_status`, `void_sale`); cross-store delta routing via `platform/sync`.
- **UUIDv7 migration (ADR #6 Phase 2)**: All 158 `Uuid::new_v4()` calls replaced with `Uuid::now_v7()` for time-ordered IDs; `oz_core::new_id()` helper added; `uuid` crate v7 feature enabled workspace-wide.
- **Multi-store security hardening (ADR #4 Phase 2, ADR #6)**: Data scoping columns (`store_id`/`warehouse_id`) on 15+ tables with compound B-Tree indexes (migration 069); `ON DELETE RESTRICT` on `store_profiles` foreign keys (migration 066); `FastPINOverlay` for shared touchscreen user switching with store isolation.
- **Scoped real-time event bus (ADR #8)**: `store_id` added to `SaleCompleted` and `CourseFired` events; KDS store-level filtering (legacy nulls pass through, matching stores pass through, mismatched stores dropped); defense-in-depth multi-store isolation for real-time events.
- **License server (ADR #9)**: PocketBase-based license server with RSA-2048 PKCS1v15 signing, rate limiting, and collections schema (`licenses`, `devices`, `audit_log`); Go-based license server binary (`apps/license-server/`) with activate/renew/status/expiry endpoints, `/api/health` readiness probe, and `normalizePEM`/`wrapPEM` PEM key normalization for single-line env var keys; RSA-2048 license verification + HTTP client in `oz-core` (`reqwest`, `store_subscription`); production multi-stage Dockerfile with CGO + healthcheck for PocketBase; Northflank deployment guide, key generation scripts (PowerShell + Bash), and SCHEMA.md collection documentation.
- **UI design token system**: 88+ non-existent tokens fixed across 33 CSS files; 90+ mismatched CSS fallbacks corrected; hardcoded colors replaced with design tokens across all screens (Login, Retail POS, KDS, Loyalty, Shift Management, EOD Report, Void Orders, Suppliers, Staff Management, Promotions, Offline Queue, and more); CSS token scanner scripts (`scan-css-tokens.py`, `fix-css-fallbacks.py`, `fix-non-existent-tokens.py`).
- **Tooltip component**: React `Tooltip` component with theme-aware colors; integrated into StatusBar, ThemeToggle, RoleBadge logout button, and sidebar collapse button; Tooltip Preview showcase page.
- **Currency auto-detection**: USD/IDR seeded in migration 006; default currency auto-detected from system locale; currency picker in setup wizard.
- **Test coverage**: Go license server test suite with 84+ tests (handleActivate 92.6%, handleStatus 100%, handleRenew 90.5%, total 85.5%) covering handler integration, rate limiting, brute-force protection, and misconfiguration error paths; front-end test suite grew from 103 to 112 test files and 1539 to 1658 tests with 9 new test files (useWorkspaceNav, useToast, useAnimatedToastQueue, ScaleIndicator, MultiStoreDashboardScreen, useTerminalProfile, useFullscreen, AppearanceSettings, DesignSystem).
- **Fast build configuration**: `sccache` + 32-thread Cargo config for local dev; `mold`/`lld` fast linker configs for Linux and macOS.
- **Adaptive Rendering & Fluid Scaling**: Redesigned `ZoomContext` to provide fluid typography scaling using `window.innerWidth` with a 1920px baseline and 14px-28px clamp; intercepted `Ctrl +/-/0` to allow keyboard zoom without fighting native browser behavior. Added `docs/UX_GUIDELINES.md` detailing the fluid typography standard.
- **Enterprise Connection Polling**: Upgraded `ConnectionStatus.tsx` to use instant OS network detection (`navigator.onLine` event listeners), exponential backoff for failed pings (up to 60s), and 30-120s randomized jitter for idle polling to prevent backend thundering herds. Added `ConnectionStatus.test.tsx` to verify OS network integration.
- **Test suite expansion (0.0.5 follow-up)**: ~263 new tests across 9 cherry-picked commits from `origin/0.0.5` — 17 `PromotionManagementScreen` render tests, 15 `useFeatures` hook tests, 37 `EodReportScreen`/`ExchangeRateScreen`/`OfflineQueueScreen` render tests, 45 hook tests (`useToast`/`useIdleTimer`/`useAnimatedModal`/`useSwipe`/`useMediaQuery`), 15 `CustomerManagementScreen` render tests, 15 Rust foundation tests (`Sku`/`LineId`/`Barcode` — Display/From/Clone/Eq/try_new/Hash/FromStr), 27 TypeScript tests (`giftCardBarcode` + `saleBarcode` UUID validation), and split commits for `SuppliersScreen` (16) + `PurchaseOrdersScreen` (19) + `GiftCardsScreen` (22) = 57 render tests and `RefundModal` (15) + `RetailOptionsScreen` (17) + `screenExtraction` (3) = 35 render tests.
- **Documentation lint coverage**: Added `#![warn(missing_docs)]` to all 9 module crates (`modules/crm`, `modules/currency`, `modules/inventory`, `modules/reporting`, `modules/sales`, `modules/settings`, `modules/staff`, `modules/tax`, `modules/terminal`) and all 4 `platform/` crates; resolved all resulting warnings.

### Changed
- **Session token migration (ADR #7)**: Every Tauri command across all modules (POS, products, inventory, sales, settings, staff, shifts, terminals, tables, workspaces, KDS, promotions, reporting) migrated from raw `user_id`/`store_id` params to session token lookup pattern with `resolve_scope()` and `resolve_store()` helpers; `Data Scope Guard` ADR documenting the pattern.
- **UI screen polish**: Final 11 screens polished with font-weight tokens, overlay tokens, and non-existent token fixes; Login screen, Retail POS, and KDS screens received comprehensive design token cleanup.
- **AGENTS.md**: Added branch-switching rule (never switch branches without user request).

### Fixed
- **TypeScript errors**: Resolved 7 TypeScript errors blocking typecheck in `StoreSwitcher.tsx`, `WorkspaceContext.tsx`, and `currency.ts`.
- **Tablet-client test WebView2 dependency**: Gated the Tauri initialization in `apps/tablet-client/src/lib.rs` behind `#[cfg(not(test))]` so the test binary no longer forces the linker to pull in `WebView2Loader.dll`. Added the same cfg gate to 5 imports (`AppError`, `AppState`, `Store`, `SyncConfig`, `Manager`) that are only used inside the gated `run()` body. This is a partial fix — the deeper resolution (target-specific Tauri dependency) is documented in the commit and deferred.
- **License activation error parsing**: Updated `LicenseActivationScreen.tsx` to properly extract error messages from `AppError` objects (and other object-based errors) in addition to `Error` class instances and raw strings.
- **PocketBase machineId compliance**: Updated the `machineId` generation in `LicenseActivationScreen` to produce exactly 15 lowercase alphanumeric characters, matching PocketBase's ID constraint.
- **TypeScript `noPropertyAccessFromIndexSignature` (TS4111)**: Changed `(err as Record<string, unknown>).message` to bracket notation `['message']` in `LicenseActivationScreen.tsx` and 4 test files (`useToast.test.tsx`, `useMediaQuery.test.ts`, `useSwipe.test.ts`, `CustomerManagementScreen.test.tsx`) to satisfy the strict index-signature access rule.
- **`scripts/check.sh` fallout (29/30 passing)**: Ran the full local CI mirror and fixed 4 clippy lints + formatting drift introduced by the batch-5/6 test additions: `clippy::collapsible_if` in `desktop-client/commands/license.rs` (collapsed nested if-let with `&&` guard); `clippy::unused_imports` for the 5 Tauri imports in `tablet-client`; `clippy::dead_code` + `clippy::unnecessary_literal_unwrap` (2 sites) in `foundation/contracts.rs` (replaced `unwrap_err()` with `let Err(err) = result else { panic!(...) };`); `clippy::clone_on_copy` in `foundation/sku.rs` (`#[allow]` on the clone-and-copy test since the `.clone()` IS the behavior under test). Ran `cargo fmt --all` to fix resulting whitespace drift. Known limitation: step 30 (`cargo test -p oz-pos-app`) still fails with `STATUS_ENTRYPOINT_NOT_FOUND` on Windows due to the same pre-existing Tauri crate dependency issue.
- **Stale `verify_signature()` calls**: Removed stray argument from `verify_signature()` in workspace commands.
- **CI build configuration**: Commented out fast linker configs (`mold`/`ld64.lld`) for CI; fixed sccache rustc-wrapper config for CI.
- **Vite config**: Fixed path aliases and test assertions for CI compatibility.
- **Fluent imports**: Updated `FluentBundle`/`FluentResource` imports from `@fluent/bundle`.
- **Test setup**: Fixed `currency_integration` tests for migration 006 seed (USD + IDR); restored `last_accessed_at` in migration 066; seeded `store_profiles` in migration 025; added missing `default_currency` field in `CompleteSetupArgs` test initializer.
- **Workspace type DTO**: Removed deprecated attribute from `WorkspaceTypeDto`, resolving 14 pre-existing Clippy warnings.
- **Documentation**: Fixed `WHITEPAPER.md` case sensitivity; moved `ARCHITECTURE.md`, `ROADMAP.md`, and `WHITEPAPER.md` into `docs/`.
- **License server**: Fixed Docker Go version from non-existent 1.26.3 to 1.25-alpine with toolchain pin; `normalizePEM` handles single-line PEM keys in env vars (Northflank strips newlines); `wrapPEM` strips whitespace from raw base64 before re-wrapping; removed conflicting duplicate `/api/health` route.
- **UI Layout Scaling**: Fixed `LicenseActivationScreen.css` breaking layout severely at high resolutions by converting hardcoded `500px` `max-width` to `31.25rem`.

## [0.0.4] — 2026-07-10

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

- **UI test & lint quality**: Resolved Vitest `exit code 1` on Node 24 CI by fixing invalid DOM nesting (`<span>` inside `<option>` across `PromotionManagementScreen`) and filtering React/Node 24 console warnings (`validateDOMNesting`, `punycode` deprecation, `act()`/`flushSync` warnings, and `@fluent/react` missing-key noise in `test-setup.ts` and `vite.config.ts`); fixed subshell pathing for `tee ui/vitest-output.log` in `.github/workflows/ci.yml` and `release.yml`; resolved all 15 React Hook `exhaustive-deps` warnings and all 5 fast-refresh/import type annotations in `ui/` (`vite.config.d.ts`, `LocaleContext`, `useToast`, `ThemeProvider`, `Toast`), achieving 0 ESLint errors and 0 warnings.

### Changed
- **Node.js 24 migration**: Migrated UI build and CI test environments (`ci.yml`, `release.yml`, and `ui/package.json` engines) to **Node.js 24**, aligning with local environments (`check.ps1`) and targeting Active LTS for the 2027 Q2 release window.





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

[Unreleased]: https://github.com/kardelitaitu/oz-pos/compare/v0.0.5...HEAD
[0.0.5]: https://github.com/kardelitaitu/oz-pos/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.4
[0.0.3]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.3
[0.0.2]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.2
[0.0.1]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.1
