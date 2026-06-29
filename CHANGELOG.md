# Changelog

All notable changes to OZ-POS are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **Real platform keychains**: Windows Credential Manager (`CredWriteW`/`CredReadW`/`CredDeleteW`), macOS Keychain (`security-framework`), Linux Secret Service (`zbus`) — all three platform backends for the `Keyring` trait are now implemented.
- **Sync HTTP transport**: `sync_client.rs` now performs an actual `reqwest` blocking POST to the configured server URL instead of logging "would sync".
- **Tax breakdown on receipt**: `LineItem` carries `tax_amount: Option<Money>`, receipt shows per-line `Tax: $X.XX` inline when `show_tax` is enabled.
- **8 tax computation unit tests**: no rates, default exclusive, default inclusive, product-level wins, category-level wins, multi-line, persisted after create, empty sale.
- **122 UI tests passing** (was 113): fixed 9 test failures across `SalesDashboardScreen`, `TaxConfigurationScreen`, and `StaffManagementScreen` caused by widget-registry architecture and Fluent bidi isolate characters in aria-labels.
- **`oz-core` Cargo.toml**: `sync-http` feature (enabled by default) gates the `reqwest` dependency.

### Changed
- `SalesDashboardScreen` is now widget-based — widget registration (`registerSalesWidgets`) must be called before rendering.
- `TaxConfigurationScreen` requires `list_categories` and `list_category_tax_rates` invoke mocks in tests.
- `rust-toolchain.toml`: restored comment accuracy (channel is `stable`, MSRV floor is 1.88 in `Cargo.toml`).

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

[Unreleased]: https://github.com/kardelitaitu/oz-pos/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/kardelitaitu/oz-pos/releases/tag/v0.0.1
