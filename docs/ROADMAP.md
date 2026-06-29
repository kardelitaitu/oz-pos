# OZ-POS — Roadmap

> *Small codebase. Limitless possibilities.*

This document defines the phased delivery plan for OZ-POS. Each phase has a clear goal, an ordered task list, and acceptance criteria. Phases build on one another — no phase begins until the previous one's criteria are met.

**Status legend:**
- `[x]` Completed
- `[/]` In progress
- `[ ]` Planned

---

## Overview

| Phase | Name | Target | Goal |
|-------|------|--------|------|
| **1** | Foundation & MVP | Month 1–2 | Working POS terminal: scan, sell, receipt |
| **2** | Hardening | Month 3–4 | Secure, tested, deployable on all platforms |
| **3** | Transactions & Staff | Month 5–6 | Full transaction lifecycle + staff management |
| **4** | Scaling | Month 7–8 | Multi-store, multi-terminal, payment gateways |
| **5** | Intelligence | Month 9–10 | Reporting, analytics, dashboards |
| **6** | Ecosystem | Month 11+ | Plugins, marketplace, advanced features |

---

## Phase 1 — Foundation & MVP
> **Goal:** A working, minimal POS terminal that can scan a barcode and complete a sale. The Setup Wizard gates everything.

### Infrastructure
- [x] Project repository: `oz-pos`
- [x] Cargo workspace with `oz-*` crate naming convention
- [x] Architecture & whitepaper documentation
- [x] `AGENTS.md` coding standards & rules
- [x] `Cargo.toml` workspace definition with all crate members
- [x] GitHub repository init, branch policy (`feat/`, `fix/`, `docs/`, `chore/`)

### Feature Flag System
- [x] `Feature` enum declared in `oz-core` (all 32 toggleable features)
- [x] `is_enabled()`, `enable()`, `disable()` helpers in `oz-core`
- [x] Feature flags stored in `settings` table as `feature.<name>` rows
- [x] Feature dependency resolution (`dependencies()` fn + auto-enable)
- [x] Store presets: **Simple Retail**, **Restaurant**, **Full Store**, **Custom**
- [x] Property-based tests (proptest) for dependency invariants, return values, serialization
- [x] 47+ comprehensive unit tests for all dependency declarations and edge cases
- [x] Doc-test on `enable()` demonstrating auto-enable behavior

### Setup Wizard (UI — First Run)
- [x] 8-step Setup Wizard UI (Tauri, React/TS)
  - Step 1: Store type / preset selection
  - Step 2: Payment methods (cash, card, multi-currency)
  - Step 3: Products (barcode, variants, inventory tracking)
  - Step 4: Staff (login, roles, shifts)
  - Step 5: Hardware (printer, cash drawer, customer display)
  - Step 6: Business rules (discounts, tax, loyalty)
  - Step 7: Data & reporting (reports, export/import, cloud sync)
  - Step 8: Review & confirm
- [x] Wizard skipped on subsequent launches (flags already set)
- [x] `useFeatures()` hook in React — UI conditionally renders based on active flags
- [x] `get_enabled_features` Tauri IPC command returning kebab-case feature keys
- [x] Feature-gated sidebar navigation — nav items hidden when required feature is disabled

### oz-core — Data Models & Engine
- [x] SQLite schema: `products`, `categories`, `sales`, `sale_lines`
- [x] SQLite schema: `settings`
- [x] SQLite schema: `currency`, `exchange_rate`
- [x] SQLite schema: `customers`, `users`, `roles`
- [x] `Money` struct (integer minor units + `Currency` reference)
- [x] `Currency` struct + ISO-4217 validation
- [x] `Customer` domain type with builder pattern (new, with_email, with_phone)
- [x] `User`/`Role` domain types with builtin role constants (owner, manager, cashier)
- [x] ISO-4217 seed data (39 currencies + 3 roles + admin user in `oz-cli init-db`)
- [x] `Product` domain type with builder pattern (new, with_category, with_barcode)
- [x] `Category` domain type (id, name, colour)
- [x] `Inventory` domain type (product_id, qty, updated_at) + is_in_stock, adjust_qty
- [x] `Sale`/`SaleLine` domain types with state machine `Pending → Active → Completed | Voided`
- [x] `Settings` store: typed key-value access + feature persistence
- [x] `Store<'a>` database facade: typed CRUD for Product, Category, Inventory, Settings
- [x] ACID write wrapper: all multi-statement writes inside explicit transactions
- [x] `updated_at` auto-update via SQLite `DEFAULT (strftime(...))` on mutable tables

### oz-hal — Hardware
- [x] `BarcodeScanner`, `ReceiptPrinter`, `CashDrawer` async trait definitions
- [x] `DriverRegistry` for lookup/injection per device category
- [x] USB barcode scanner driver (HID)
- [x] Serial barcode scanner driver (stub)
- [x] Receipt printer driver (USB, stub output)
- [x] Bluetooth (SPP) receipt printer driver
- [x] TCP/network receipt printer driver (port 9100, raw ESC/POS)
- [x] Shared ESC/POS formatting module (`escpos.rs`)
- [x] Mock HAL driver for unit tests (`hal/src/drivers/mock.rs`)

### UI — Design System & Component Library
- [x] CSS design tokens: colour palette, spacing scale, border-radius, shadows (`styles/tokens.css`)
- [x] CSS reset (`styles/reset.css`)
- [x] Shared component styles (`styles/components.css`)
- [x] Dark mode + light mode — system-preference aware, user-toggleable (`ThemeProvider` + `ThemeToggle`)
- [x] Core component library: `Button`, `Input`, `Card`, `Modal`, `Badge`, `Toast`, `Spinner`, `Skeleton`
- [x] Components: `EmptyState`, `ErrorState` — consistent empty/error patterns
- [x] Micro-animations: button press, modal fade/slide, toast slide-in, skeleton pulse, setup completion bounce
- [x] ARIA labels and keyboard support on all interactive elements
- [x] Focus trap + Escape-to-close in Modal
- [x] Design System showcase page at `/design` route

### UI — Core Checkout Flow
- [x] Product lookup screen (barcode scan → product card) — `ProductLookupScreen`
- [x] Product card with name, price, stock badge, category chip, add-to-cart
- [x] Category filter chips (radiogroup pattern)
- [x] Barcode scan input with scan button and Enter-key support
- [x] Shopping cart (add/remove/quantity) — inline cart panel in `PosScreen`
- [x] Cart line items with quantity stepper (minus/plus), remove button, line total
- [x] Subtotal and Pay button in cart footer
- [x] Payment modal: Cash / Card / Other payment methods
- [x] Cash tendered input with change calculation and insufficient-amount warning
- [x] Sale completion via IPC (`startSale` → `addLine` → `completeSale`)
- [x] Receipt printing via IPC (`printSalesReceipt`)
- [x] Auto-close after successful payment with change-due display
- [x] Global navigation sidebar with SVG icons (`AppLayout`)

### UI — Setup Wizard
- [x] Preset selection cards (🛒 Retail, 🍽️ Restaurant, 🏪 Full Store, ⚙️ Custom)
- [x] Feature toggle switches across 6 categories (Payments, Products, Staff, Hardware, Business Rules, Data)
- [x] Step progress indicator with completed/active/pending dots
- [x] Review step with feature tag cloud (enabled + disabled)
- [x] Completion screen with checkmark bounce animation
- [x] Back / Next / Skip navigation with memoized callbacks

### UI — Settings & Product Management
- [x] Settings UI — store name, address, tax ID, receipt preferences (`SettingsPage`)
- [x] Product Management UI — data table with add/edit/delete (`ProductManagementScreen`)
- [x] Barcode field in product form
- [x] Category Management UI — standalone colour-coded category list with add/delete + colour picker
- [x] Category management IPC commands: `list_categories`, `create_category`, `delete_category`
- [x] `useFeatures()` hook — sidebar nav hides items for disabled features
- [x] `get_enabled_features` IPC endpoint — backend returns enabled feature keys
- [x] Feature-gated sidebar navigation — categories route hidden behind `categories-enabled` flag

### Database
- [x] `migrations/001_sales.sql` + `002_products.sql` + `003_barcode.sql` + `004_sale_status.sql`
- [x] `oz-core` migration runner (embedded via `include_str!`, run on startup)
- [x] Domain-to-schema mapping: `Product`, `Category`, `Inventory`, `Sale`, `SaleLine`, `Settings`
- [x] `Cart` (in-memory) → `Sale` (persisted) pipeline with `Sale::from_cart()`
- [x] `oz-cli init-db` — seeds default settings + preset flags + feature flags
- [x] `oz-cli init-db` — seeds 39 ISO-4217 currencies, 3 built-in roles (owner/manager/cashier), and admin user

### API — REST Endpoints (Phase 1 MVP)
- [x] `GET /api/v1/health` — server health + version
- [x] `POST /api/v1/tokens` — JWT token creation (label + expiry)
- [x] JWT auth middleware on all protected routes
- [x] `GET /api/v1/products` — list products with category name + stock (LEFT JOIN)
- [x] `GET /api/v1/products/{sku}` — product detail by SKU
- [x] `POST /api/v1/products` — create product (validates, inserts, optional inventory)
- [x] `PATCH /api/v1/products/{sku}/stock` — adjust stock with transaction + checked arithmetic
- [x] `GET /api/v1/categories` — list categories ordered by name
- [x] CORS enabled, tracing/logging layer
- [x] Integration tests with in-memory SQLite + seeded data

### Acceptance Criteria
- [x] Full checkout flow validated: scan barcode → add to cart → pay → print receipt
- [x] Setup Wizard completes and persists flags to SQLite
- [x] UI hides all inactive features (e.g., no loyalty tab in Simple Retail) via `useFeatures()` hook + feature-gated nav
- [x] Dark mode and light mode both render without visual glitches
- [x] Design tokens applied consistently — no hardcoded hex colours in components
- [x] `cargo test` passes across all crates (250+ tests, 0 failed)
- [x] `cargo clippy -- -D warnings` passes with zero warnings
- [x] 250+ unit tests across `oz-core` + `oz-api` + `oz-hal` + `oz-pos-app`
- [x] Data Management UI wired to real IPC (backup, export/import .ozpkg)
- [x] `oz-cli import-ozpkg` writes data to DB (products, categories, sales, customers, users, settings)
- [x] StaffLoginScreen supports hardware keyboard PIN entry (digits, Backspace, Enter, Escape)
- [ ] App launches on Windows and Linux

---

## Phase 2 — Hardening
> **Goal:** Secure, fully tested, and deployable on all four target platforms with a CI/CD pipeline.

### oz-security
- [x] OS key-ring abstraction (`Keyring` trait + `InMemoryKeyring` + platform stubs)
    - Windows: `WindowsCredentialManager` (stub)
    - Linux: `LibSecretKeyring` (stub)
    - macOS: `MacOsKeychain` (stub)
    - Fallback: `InMemoryKeyring` with 5 unit tests
- [x] TLS configuration helpers for cloud sync traffic (`TlsConfig` + builder)
    - Certificate/key/CA path loading with existence validation
    - ALPN protocol support, insecure-skip-verify option (dev only)
    - 7 unit tests for builder, validation, file loading
- [x] PCI-DSS checklist helpers (`mask::mask_pan`, `mask::is_valid_pan`, etc.)
    - PAN masking (first 6 + last 4) per PCI-DSS 3.3
    - Luhn validation for PAN format checks
    - Cardholder name masking, CVV masking
    - 15 unit tests
- [x] `docs/security/PCI-DSS_CHECKLIST.md` — full PCI-DSS v4.0 compliance checklist
- [x] `.env.example` template for development secrets

### oz-logging
- [x] `tracing` + `tracing-subscriber` initialiser (`oz_logging::init()`)
- [x] JSON log formatter (`oz_logging::init_json()`) — ELK/Loki compatible
- [x] File writer with rotation (`oz_logging::init_with_file()`, `oz_logging::init_json_with_file()`)
    - Uses `tracing-appender` for hourly rolling files
    - Spawns background cleanup thread for log retention (configurable days)
- [x] Syslog output (Linux) — `oz_logging::syslog::init_syslog()`
    - Uses `libc` FFI for syslog API
    - Combined subscriber: stdout + syslog via `tracing_subscriber::registry()`
    - Configurable facility (local0–local7, daemon, user, etc.)
- [x] Windows Event Log output — `oz_logging::eventlog::init_eventlog()`
    - Uses `OutputDebugStringW` via windows-sys FFI
    - Combined subscriber: stdout + debug output via registry()
- [x] Shared `MessageVisitor` for field formatting (extracted to `visitor.rs`)

### Testing
- [x] Unit test `#[cfg(test)]` blocks in all `oz-*` crates
- [x] Integration tests with mock HAL drivers (25 tests in `oz-hal/tests/mock_integration.rs`)
- [x] Front-end: Vitest + React Testing Library (`ui/src/__tests__/`)
- [x] `eslint-plugin-jsx-a11y` enabled in `ui/.eslintrc.cjs`
- [ ] Test coverage target: ≥ 80% on `oz-core`, `oz-hal`, `oz-lua` (requires tarpaulin)

### What's left in Phase 2
- ~1,100+ tests across all crates, all passing — coverage measurement via tarpaulin
- Build-and-install validation on all target platforms
- App auto-updates from a published GitHub release

### CI/CD
- [x] `.github/workflows/ci.yml`: lint → test → Tauri bundle
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --workspace`
  - `npm run lint` + `npm run test` in `ui/`
  - Tauri build for `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`
  - `.github/workflows/security.yml`: weekly `cargo audit` + `cargo deny`
- [x] `.github/workflows/release.yml`: tag → build all targets → draft GitHub release
  - Verify job: fmt, clippy, tests (cargo + npm)
  - Build matrix: ubuntu-latest, windows-latest, macos-latest
  - Platform bundles: .deb + .AppImage (Linux), .msi + .nsis (Windows), .dmg (macOS)
  - Artifact upload + softprops/action-gh-release (draft)
  - `latest.json` metadata generation for auto-update

### Data Management
- [x] `oz-cli backup` — raw SQLite snapshot (`.db` file)
- [x] `oz-cli restore` — restore from snapshot
- [x] `oz-cli export` — encrypted `.ozpkg` (Argon2id + AES-256-GCM + zstd)
  - Flags: `--types`, `--password`
- [x] `oz-cli import` — decrypt and apply `.ozpkg`
  - Flags: `--dry-run`, `--password`
- [x] Feature flags embedded in `.ozpkg` plaintext metadata

### Updates & Packaging
- [x] Tauri auto-update (`tauri.conf.json` updater section → GitHub releases)
  - `tauri-plugin-updater` registered in `lib.rs` + `Cargo.toml`
  - Updater signing key pair generated (`oz-pos-updater.key` + `.pub`)
  - Public key set in `tauri.conf.json`
  - Endpoint: GitHub releases `latest.json`
- [x] Windows: NSIS installer (configured in `tauri.conf.json`)
- [x] Windows: MSI installer (WiX configured via `bundle.targets: "all"`)
- [x] Linux: `.deb` + `.AppImage` packages
- [x] macOS: `.dmg` package
- [x] `packaging/README.md` — packaging overview and build guide
- [x] `packaging/linux/oz-pos.desktop` — freedesktop entry
- [x] `packaging/linux/deb/postinst` — Debian post-install script
- [x] `packaging/linux/deb/prerm` — Debian pre-removal script

### UI / UX — Data Management & Feature Toggle Screens
- [x] **Data Management screen** (Settings → Data)
  - Export wizard: select data types (checkboxes + select all/none), date range picker, encryption password with confirmation, progress bar, completion state
  - Import wizard: file dropzone, native file dialog (`@tauri-apps/plugin-dialog`), plaintext metadata preview, decryption password, dry-run diff table, progress bar, completion state
  - Backup status panel: last backup timestamp + size, one-click create backup button with loading state
  - All flows wired to real Tauri IPC commands (`get_backup_status`, `create_backup`, `export_data`, `import_preview`, `import_data`)
  - Accessible tabbed layout with ARIA roles, full dark-mode compatible CSS
- [x] **Feature Toggle screen** (Settings → Features) — master on/off panel for all 32 flags post-setup
  - `list_all_features` + `set_feature` IPC commands with automatic dependency resolution
  - Grouped by category with toggle switches, dependency info, and toast notifications
  - Route: Features (lightning bolt icon sidebar nav, always visible)
- [x] **Update notification banner** (`UpdateBanner` component)
    - Auto-checks via `@tauri-apps/plugin-updater` on mount
    - Dismissible banner with "Install" action button
    - Slide-in animation, dark mode support
    - Graceful fallback when Tauri not available

### Acceptance Criteria
- [x] CI pipeline passes on every PR
- [x] Export + import round-trip: data survives encrypt → decrypt → import (verified with 7 ozpkg tests)
- [x] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] App auto-updates from a published GitHub release (requires actual release)
- [ ] Installable via MSI on Windows and `.deb` on Ubuntu (requires actual build)
- [x] Data Management UI: export and import wizard screens complete

### What's left in Phase 2
- Unit test `#[cfg(test)]` blocks in all `oz-*` crates
- Integration tests with mock HAL drivers
- Test coverage target: ≥ 80% on `oz-core`, `oz-hal`, `oz-lua`
- Build-and-install validation on all target platforms

---

## Phase 3 — Transactions & Staff
> **Goal:** Full transaction lifecycle (void, refund, hold), staff roles, shifts, audit trail, and tax rules.

### Transaction Lifecycle
- [x] Audit log SQL migration + domain type (`010_audit_log.sql`, `audit.rs`)
- [x] Store methods: `log_audit`, `list_audit_entries`, `void_sale` (atomic tx with stock restoration)
- [x] `void_sale` Tauri IPC command (`src-tauri/src/commands/sales.rs`)
- [x] **Void Sale UI** — Orders screen with search, status filters, detail view, reason picker, void confirmation
- [x] Refund / return flow (partial or full, linked to original order) — `RefundModal.tsx`, `SalesHistoryScreen.tsx` integration, previous refunds display
- [x] Hold order (park a sale, resume later — multiple holds simultaneously)
- [x] Split bill (divide order across multiple payment methods or customers)
- [x] End-of-Day (EOD) report: cash tally, payment breakdown, shift summary

### Staff & Auth
- [x] `StaffLogin` feature: argon2id PIN hashing + verification (`oz_core::auth`)
- [x] `staff_login` IPC command (username lookup, PIN verify, role resolution)
- [x] `list_staff`, `create_staff`, `update_staff` IPC commands with PIN hashing on create
- [x] `AuthContext` + `useAuth` hook — React context for session state, login/logout, isManager/isOwner
- [x] **Staff Login UI** — full-screen PIN pad (2-step: username → numeric PIN entry), auto-submit at 6 digits or manual submit for shorter PINs
- [x] **Role badge** — sidebar user info with avatar letter, display name, colour-coded role, logout button
- [x] **Permission denied screen** — reusable `PermissionDenied` component with lock icon and role requirement
- [x] `StaffRoles` feature: owner, manager, cashier permission model (backend role-based route gating)
- [x] `ShiftManagement` feature: open/close shift with opening balance
- [x] Cash drawer reconciliation: expected vs. actual cash at close
- [x] `AuditLog` UI: view and filter audit log entries

### Tax Engine
- [x] Tax inclusive vs. exclusive toggle (per rate)
- [x] Tax rate per category (via category_taxes junction table)
- [x] Multi-rate support (e.g., 0%, 7%, 10% on different product types)
- [x] Tax breakdown on receipt and in order records

### oz-lua — Scripting Runtime
- [x] Embed `rlua` Lua VM in `oz-lua`
- [x] Expose `apply_discount()`, `calc_line_tax()`, `validate_order()` to Lua
- [x] Merchant Lua scripts loaded from `scripts/` at runtime (`load_dir`)
- [x] Lua sandbox: no filesystem or network access from scripts
- [x] Example Lua scripts in `scripts/examples/`

### Hardware Expansion
- [x] Bluetooth barcode scanner driver
- [x] Cash drawer trigger (via receipt printer GPIO)
- [x] Customer display driver (secondary screen, shows cart total)

### UI / UX — Staff, Transaction & Management Screens

**Staff & Auth**
- [x] **Staff Login screen** — full-screen PIN pad (2-step: username → PIN entry with numeric keypad)
- [x] **Role badge** — sidebar user info with avatar letter, name, role colour, logout button
- [x] **Staff Management IPC** — `list_staff`, `create_staff`, `update_staff` commands with argon2 PIN hashing
- [x] **Permission denied screen** — reusable lock-icon component with role requirement message
- [x] **Staff Management UI** — table with avatar, role badges, add/edit modal with PIN hashing, deactivate/restore toggle

**Transaction Screens**
- [x] **Void Sale UI** — Orders screen with search, status filter chips, detail view, reason picker (8 presets + custom), void confirmation with success/error feedback
- [x] **Refund / Return UI** — select items to refund (partial or full), amount preview, confirm
- [x] **Hold Order UI** — name/tag a held order, resume from holds list
- [x] **Split Bill UI** — assign amounts per payment method (cash/card/other) with even-split, add/remove, remaining tracker
- [x] **Order History UI** — searchable list with filters (date, status, cashier), tap to view detail
- [x] **Order Detail UI** — full receipt view with void/refund actions for managers

**Shift & Cash**
- [x] **Open Shift screen** — enter opening cash balance, confirm
- [x] **Close Shift screen** — count cash by denomination, compare to expected, print EOD summary
- [x] **EOD Report screen** — sales total, cash, card, voids, discounts breakdown, shift data, print

**Business Rule UI**
- [x] **Discount UI** — apply % or fixed discount to whole cart or individual line items
- [x] **Tax Configuration UI** — inclusive/exclusive toggle per rate, type column in table, category tax rates section
- [x] **Customer Management UI** — searchable table with avatar initials, add/edit modal (name/email/phone/notes), delete
- [x] **Inventory Adjustment UI** — manual stock-in / stock-out with reason field

### Acceptance Criteria
- [x] Void and refund flows update stock and produce audit entries
- [x] Shift open/close persists cash reconciliation report
- [x] A Lua discount script applies correctly at checkout
- [x] Audit log entries are write-once (no UPDATE/DELETE allowed on `audit_log`)
- [x] RBAC: cashier cannot access manager-only screens
- [x] PIN login screen renders correctly on all target platforms (hardware keyboard support: digits, Backspace, Enter, Escape; touch targets ≥ 56×72px; ARIA labels)

---

## Phase 4 — Scaling
> **Goal:** Multi-store, multi-terminal, cloud sync, payment gateways, and mobile builds.

### Cloud Sync (Optional On-Feature)
- [x] SQLite outbox pattern (`offline_queue` table in `oz-core` + `platform-sync` queue/transport/replication layer)
- [ ] Background sync daemon: outbox → PostgreSQL (via `tokio-postgres`)
- [x] Conflict resolution strategy (last-write-wins with timestamp in `platform/sync/src/conflict.rs`)
- [ ] Cloud DB add-on: AWS RDS or Azure Database for PostgreSQL
- [ ] Redis cache: product look-ups, pricing rules, inventory pub/sub

### Multi-Store & Multi-Terminal
- [x] Store entity: each store has its own settings + feature flags
- [x] Store profile seeded on first startup + IPC commands (list, get, create, update, set-primary, delete)
- [ ] Multi-store management UI (owner view across all locations)
- [ ] Multi-terminal: terminals in the same store share inventory via cloud sync
- [ ] Per-terminal feature overrides (e.g., terminal A has KDS, terminal B does not)

### oz-payment
- [x] `PaymentProcessor` trait definition
- [x] Mock payment processor (for testing and offline demo)
- [x] Stripe integration (card present + card not present)
- [ ] Square integration (optional)
- [ ] QRIS / local payment gateway support (Indonesian market)
- [ ] Payment result stored in `payments` table linked to `order_id`

### Multi-Currency
- [ ] `exchange_rate` table populated by background sync from external API
- [ ] Currency selector in checkout UI (when `MultiCurrency` flag enabled)
- [ ] Receipts show both charge currency and base currency

### Mobile Builds
- [ ] Android tablet build (Tauri mobile → APK, signed)
- [ ] iPad build (Tauri mobile → `.ipa`, TestFlight distribution)
- [ ] Touch-optimised UI layout for tablet screen sizes
- [ ] `packaging/mobile/README.md` — mobile build guide

### UI / UX — Responsive, Mobile & Multi-Store Screens

**Responsive & Adaptive Layout**
- [ ] Breakpoint system: desktop (≥1024px), tablet landscape (768–1023px), tablet portrait (<768px)
- [ ] Checkout layout: split-view on desktop, stacked on tablet portrait
- [ ] Navigation: sidebar on desktop → collapsible drawer on tablet → bottom bar on mobile
- [ ] Touch targets: minimum 44×44px on all interactive elements (Apple HIG / Material)
- [ ] Swipe gestures: swipe cart item to remove, swipe order to void (tablet)

**Multi-Store & Multi-Terminal**
- [ ] **Store Switcher** — dropdown in header showing active store + quick-switch
- [ ] **Multi-Store Dashboard** — owner view: all stores, revenue per location, active terminals
- [ ] **Terminal Status UI** — which terminals are online/offline in a store
- [ ] **Per-Terminal Settings UI** — override feature flags for a specific terminal

**Payment & Currency UI**
- [ ] **Payment Gateway status badge** — online / offline / degraded indicator
- [ ] **QRIS QR code display** — full-screen QR on checkout, auto-dismiss on payment confirm
- [ ] **Currency selector** — flag + ISO code dropdown at checkout when MultiCurrency enabled
- [ ] **Exchange rate notice** — show rate used and timestamp on receipt

**Hardware Integration UI**
- [x] **Customer display wired to PosScreen** — `useCustomerDisplay` hook auto-detects the first registered display, shows cart total + item count on two 20-char lines, clears on payment complete or cart empty

### Acceptance Criteria
- [ ] Cloud sync: a product updated on terminal A appears on terminal B within 5 seconds
- [ ] Payment via Stripe sandbox succeeds end-to-end
- [ ] App installs and runs on Android tablet (Android 10+)
- [ ] App installs and runs on iPad (iPadOS 16+)
- [ ] Checkout layout adapts correctly at all defined breakpoints
- [ ] Touch targets pass Apple HIG 44px minimum on tablet builds

---

## Phase 5 — Intelligence
> **Goal:** Actionable merchant insights, dashboards, analytics, and i18n.

### oz-reporting
- [ ] Daily / weekly / monthly sales summary queries
- [ ] Inventory low-stock alerts and reorder notifications
- [ ] Top products, category breakdown, hourly heatmap
- [ ] CSV export: `sales_report.csv`, `inventory_report.csv`
- [ ] Dashboard UI: revenue chart, top products panel, inventory status widget

### Analytics (Optional On-Feature)
- [ ] Analytics export to cloud warehouse (BigQuery / Snowflake)
- [ ] Scheduled report delivery (email PDF)
- [ ] Custom report builder (drag-and-drop columns)

### Accessibility & i18n
- [ ] WCAG-2.1 AA audit on all UI screens
- [x] ARIA labels on all interactive elements
- [ ] `ui/src/i18n/en.ftl` — English locale (all strings)
- [ ] `ui/src/i18n/id.ftl` — Bahasa Indonesia locale
- [ ] `ui/src/i18n/th.ftl` — Thai locale
- [ ] `@fluent/react` integration — no hardcoded strings in JSX
- [ ] `docs/a11y.md` — accessibility compliance checklist

### oz-reporting — Performance & Profiling
- [ ] `tokio-console` integration macros
- [ ] `cargo flamegraph` helpers
- [ ] Benchmark suite: barcode lookup < 1 ms, transaction commit < 5 ms
- [ ] Prometheus metrics endpoint (optional)

### UI / UX — Reports, Dashboard & i18n Screens

**Dashboard & Reports**
- [ ] **Home Dashboard** — today's revenue card, orders count, top product, low-stock alert widget
- [ ] **Sales Report screen** — line chart (revenue over time), bar chart (by category), date range filter
- [ ] **Inventory Report screen** — stock table with low-stock highlighted in amber/red, reorder button
- [ ] **Top Products screen** — ranked list with sparkline, filter by period
- [ ] **Hourly Heatmap** — grid showing busiest hours of the day/week
- [ ] **Export Report button** — one-tap CSV download from any report screen
- [ ] **Print Report button** — sends formatted report to receipt printer

**i18n UI**
- [ ] Language selector in Settings (flag + language name)
- [ ] RTL layout support scaffolded (for future Arabic/Hebrew locales)
- [ ] All number, date, and currency formats respect selected locale

### Acceptance Criteria
- [ ] Dashboard loads and renders with real SQLite data
- [ ] Lighthouse a11y score ≥ 90 on all pages
- [ ] UI fully translated in English + Bahasa Indonesia
- [ ] Barcode lookup benchmark consistently < 1 ms on all target platforms
- [ ] All report screens render correctly with empty data (no crashes, good empty states)

---

## Phase 6 — Ecosystem
> **Goal:** Plugin marketplace, advanced merchant features, and long-term extensibility.

### Advanced Business Features
- [ ] Loyalty program: customer points, tiers, redemption
- [ ] Promotions engine: buy-X-get-Y, % off, fixed discount, time-limited
- [ ] Product bundles (sell multiple SKUs as one item)
- [ ] Kitchen Display System (KDS) — restaurant order routing
- [ ] Self-service kiosk mode (locked-down fullscreen UI)
- [ ] Table management UI (restaurant floor plan)

### Plugin System
- [ ] Stable plugin API for third-party HAL drivers
- [ ] Plugin manifest format (`plugin.toml`)
- [ ] Plugin sandbox: Lua-based, no unsafe Rust from plugins
- [ ] Plugin discovery and hot-reload
- [ ] Developer docs: `docs/plugin-guide.md`

### Developer Experience
- [ ] `cargo doc` generated and hosted on GitHub Pages
- [ ] `CONTRIBUTING.md` — contribution guide, PR template
- [ ] `docs/quickstart.md` — local dev setup
- [ ] Example Lua scripts in `scripts/examples/`
- [ ] Example custom HAL driver in `oz-hal/examples/`

### Future Research
- [ ] AI-driven product recommendations (demand forecasting)
- [ ] Offline-first sync with CRDTs for conflict-free replication
- [ ] Voice-controlled checkout (accessibility extension)

### UI / UX — Advanced Screens & Theming

**Advanced Business Screens**
- [ ] **Loyalty UI** — customer points balance on checkout, redeem button, tier badge
- [ ] **Promotions UI** — active promotions banner on cart, promotion management screen for managers
- [ ] **Product Bundles UI** — bundle card in product grid, items listed on receipt
- [ ] **Kitchen Display System (KDS)** — fullscreen order queue, ticket cards with status (New / In Progress / Done), tap to advance
- [ ] **Self-Service Kiosk UI** — locked-down fullscreen layout, large product grid, no nav bar, attract screen when idle
- [ ] **Table Management UI** — interactive restaurant floor plan, table status colours (Available / Occupied / Reserved), tap to open order

**Theming & White-Label**
- [ ] Merchant logo upload (shown in header, on receipts, on kiosk attract screen)
- [ ] Brand primary colour picker → applies to buttons, accents, active states across the whole UI
- [ ] Theme preview in Settings before applying
- [ ] Dark / light / system-default theme saved per device

### Acceptance Criteria
- [ ] A third-party plugin installs and a custom barcode scanner works
- [ ] Loyalty points accrue and redeem correctly at checkout
- [ ] `cargo doc` builds without warnings
- [ ] KDS shows incoming restaurant orders in real time
- [ ] Kiosk mode: no way to exit to OS without manager PIN
- [ ] Custom brand colour applies immediately across all screens

---

## On-Features — Optional Paid Add-ons

These features are available as opt-in services, billed per usage or per store.

| Add-on | Description | Phase Available |
|--------|-------------|-----------------|
| **Cloud Database** | Managed PostgreSQL / CockroachDB; auto-backup, multi-region, point-in-time restore | Phase 4 |
| **Cloud Sync** | SQLite → PostgreSQL outbox sync daemon, hosted | Phase 4 |
| **Analytics Export** | Push sales data to BigQuery / Snowflake; scheduled reports | Phase 5 |
| **Advanced Reporting** | Custom report builder with email delivery | Phase 5 |
| **Loyalty Program** | Customer points, rewards tiers, and redemption engine | Phase 6 |
| **Priority Support** | SLA-backed support tickets, dedicated onboarding, and setup assistance | Any phase |

---

## Dependency Graph

```
Phase 1 (MVP)
    └── Phase 2 (Hardening)
            └── Phase 3 (Transactions & Staff)
                    └── Phase 4 (Scaling)
                            └── Phase 5 (Intelligence)
                                    └── Phase 6 (Ecosystem)
```

On-Features can be activated at any phase once the core infrastructure is in place.

---

*Last updated: 2026-06-30.*
