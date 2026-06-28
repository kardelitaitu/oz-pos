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
- [x] Feature dependency resolution (`dependencies()` fn + auto-enable + notification)
- [x] Store presets: **Simple Retail**, **Restaurant**, **Full Store**, **Custom**

### Setup Wizard (UI — First Run)
- [ ] 8-step Setup Wizard UI (Tauri, React/TS)
  - Step 1: Store type / preset selection
  - Step 2: Payment methods (cash, card, multi-currency)
  - Step 3: Products (barcode, variants, inventory tracking)
  - Step 4: Staff (login, roles, shifts)
  - Step 5: Hardware (printer, cash drawer, customer display)
  - Step 6: Business rules (discounts, tax, loyalty)
  - Step 7: Data & reporting (reports, export/import, cloud sync)
  - Step 8: Review & confirm
- [ ] `useFeature()` hook in React — UI conditionally renders based on active flags
- [ ] Wizard skipped on subsequent launches (flags already set)

### oz-core — Data Models & Engine
- [x] SQLite schema: `products`, `categories`, `sales`, `sale_lines`
- [x] SQLite schema: `settings`
- [x] SQLite schema: `currency`, `exchange_rate`
- [x] SQLite schema: `customers`, `users`, `roles`
- [x] `Money` struct (integer minor units + `Currency` reference)
- [x] `Currency` struct + ISO-4217 validation
- [x] `Customer` domain type with builder pattern (new, with_email, with_phone)
- [x] `User`/`Role` domain types with builtin role constants (owner, manager, cashier)
- [ ] ISO-4217 seed data (gated behind `oz-cli init-db`)
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

### UI — Core Checkout Flow
- [x] Tauri v2 project scaffold (React + TypeScript + Vite)
- [x] `ui/src/api/pos.ts` — Tauri command bridge (no `invoke` in components)
- [ ] Product lookup screen (barcode scan → product card)
- [x] Shopping cart (add/remove/quantity)
  - CartScreen scaffold (display-only; interactions pending)
- [ ] Checkout screen (payment method selection, total, tax)
- [ ] Receipt view (on-screen, print trigger)
- [ ] Global navigation (hides items for disabled features)

### Database
- [x] `migrations/001_sales.sql` + `002_products.sql` + `003_barcode.sql` + `004_sale_status.sql`
- [x] `oz-core` migration runner (embedded via `include_str!`, run on startup)
- [x] Domain-to-schema mapping: `Product`, `Category`, `Inventory`, `Sale`, `SaleLine`, `Settings`
- [x] `Cart` (in-memory) → `Sale` (persisted) pipeline with `Sale::from_cart()`
- [x] `oz-cli init-db` — seeds default settings + preset flags + feature flags
- [ ] `oz-cli init-db` — seed ISO-4217 currencies + built-in roles + admin user

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

### UI / UX — Design System & Core Screens

**Design System (built first, before any screen)**
- [ ] Choose & configure Google Font (e.g. Inter or Outfit) via `index.css`
- [ ] Define CSS design tokens: colour palette, spacing scale, border-radius, shadows
- [ ] Dark mode + light mode — system-preference aware, user-toggleable
- [ ] Core component library: `Button`, `Input`, `Card`, `Modal`, `Badge`, `Toast`, `Spinner`
- [ ] Micro-animations: button press, page transitions, scan success pulse
- [ ] Loading states for all async operations (skeleton screens, not spinners alone)
- [ ] Empty states: no products found, no orders today, offline banner
- [ ] Error states: scan failure, printer offline, network error — all with recovery actions
- [ ] `AccessibleButton` and ARIA-labelled base components from day one

**Setup Wizard UI**
- [ ] 8-step animated wizard with progress bar
- [ ] Preset selection cards (illustrated: Retail 🛒, Restaurant 🍽️, Full 🏪, Custom ⚙️)
- [ ] Yes / No toggle switches — clean, large tap targets
- [ ] Review screen: feature tag cloud showing what's enabled
- [ ] Completion animation before entering the app

**Core Checkout Screens**
- [ ] **Product Lookup** — barcode scan input, search bar, product grid/list toggle
- [ ] **Product Card** — name, price, stock badge, category colour chip
- [ ] **Cart Panel** — slide-in or split-view; line items, quantity stepper, subtotal
- [ ] **Checkout Screen** — payment method tabs (Cash / Card / QRIS), total with tax breakdown
- [ ] **Receipt View** — on-screen receipt with store logo, line items, tax, payment method; print button
- [ ] **Global Navigation** — sidebar or bottom bar; hides items for disabled features

**Settings & Product Management**
- [ ] **Settings UI** — store name, logo upload, currency selector, feature toggle panel
- [ ] **Product Management UI** — data table with add/edit/delete; barcode field with scan-to-fill
- [ ] **Category Management UI** — colour-coded category list

### Acceptance Criteria
- [ ] Full checkout flow: scan barcode → add to cart → pay → print receipt
- [ ] Setup Wizard completes and persists flags to SQLite
- [ ] UI hides all inactive features (e.g., no loyalty tab in Simple Retail)
- [ ] Dark mode and light mode both render without visual glitches
- [ ] Design tokens applied consistently — no hardcoded hex colours in components
- [x] `cargo test` passes across all crates (250 tests, 0 failed)
- [x] `cargo clippy -- -D warnings` passes with zero warnings
- [x] 250 unit tests across `oz-core` (172) + `oz-api` (65) + `oz-hal` (13)
- [ ] App launches on Windows and Linux

---

## Phase 2 — Hardening
> **Goal:** Secure, fully tested, and deployable on all four target platforms with a CI/CD pipeline.

### oz-security
- [ ] OS key-ring abstraction (Windows Credential Manager, Linux Secret Service)
- [ ] TLS configuration helpers for cloud sync traffic
- [ ] PCI-DSS checklist helpers (tokenisation, encrypted field storage)
- [ ] `.env.example` template for development secrets

### oz-logging
- [x] `tracing` + `tracing-subscriber` initialiser (`oz_logging::init()`)
- [ ] JSON log formatter (ELK/Loki compatible)
- [ ] Syslog output (Linux)
- [ ] Windows Event Log output
- [ ] Log rotation & retention policy

### Testing
- [ ] Unit test `#[cfg(test)]` blocks in all `oz-*` crates
- [ ] Integration tests with mock HAL drivers
- [ ] Front-end: Jest + React Testing Library (`ui/src/__tests__/`)
- [x] `eslint-plugin-jsx-a11y` enabled in `ui/.eslintrc.cjs`
- [ ] Test coverage target: ≥ 80% on `oz-core`, `oz-hal`, `oz-lua`

### CI/CD
- [x] `.github/workflows/ci.yml`: lint → test → Tauri bundle
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test --workspace`
  - `npm run lint` + `npm run test` in `ui/`
  - Tauri build for `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`
- [ ] `.github/workflows/release.yml`: tag → build all targets → draft GitHub release

### Data Management
- [ ] `oz-cli backup` — raw SQLite snapshot (`.db` file)
- [ ] `oz-cli restore` — restore from snapshot
- [ ] `oz-cli export` — encrypted `.ozpkg` (Argon2id + AES-256-GCM + zstd)
  - Flags: `--no-orders`, `--no-customers`, `--orders-from`, `--orders-to`
- [ ] `oz-cli import` — decrypt and apply `.ozpkg`
  - Flags: `--dry-run`, `--skip-existing`, `--overwrite-settings`
- [ ] Feature flags embedded in `.ozpkg` plaintext metadata

### Updates & Packaging
- [ ] Tauri auto-update (`tauri.conf.json` updater section → GitHub releases)
- [ ] Windows: MSI installer (WiX)
- [ ] Linux: `.AppImage` + `.deb` packages
- [ ] `packaging/windows/installer.wxs`
- [ ] `packaging/linux/deb/metadata.yaml`

### UI / UX — Data Management Screens
- [ ] **Data Management screen** (Settings → Data)
  - Export wizard: checkboxes for data types, date range picker, password field, progress bar
  - Import wizard: file picker, plaintext metadata preview (no password needed), dry-run diff table, confirm button
  - Backup status: last backup timestamp, storage location
- [ ] **Feature Toggle screen** (Settings → Features) — master on/off panel for all flags post-setup
- [ ] **Update notification banner** — non-intrusive toast when a new version is available

### Acceptance Criteria
- [ ] CI pipeline passes on every PR
- [ ] Export + import round-trip: data survives encrypt → decrypt → import
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] App auto-updates from a published GitHub release
- [ ] Installable via MSI on Windows and `.deb` on Ubuntu
- [ ] Data Management UI: export and import flows complete without errors

---

## Phase 3 — Transactions & Staff
> **Goal:** Full transaction lifecycle (void, refund, hold), staff roles, shifts, audit trail, and tax rules.

### Transaction Lifecycle
- [ ] Void sale (cancels active order, restores stock)
- [ ] Refund / return flow (partial or full, linked to original order)
- [ ] Hold order (park a sale, resume later — multiple holds simultaneously)
- [ ] Split bill (divide order across multiple payment methods or customers)
- [ ] End-of-Day (EOD) report: cash tally, payment breakdown, shift summary

### Staff & Auth
- [ ] `StaffLogin` feature: cashier PIN / password
- [ ] `StaffRoles` feature: owner, manager, cashier permission model
- [ ] `ShiftManagement` feature: open/close shift with opening balance
- [ ] Cash drawer reconciliation: expected vs. actual cash at close
- [ ] `AuditLog` feature: immutable append-only log for sensitive actions
  - Actions logged: login, void, discount, refund, settings change, export

### Tax Engine
- [ ] Tax inclusive vs. exclusive toggle (per store)
- [ ] Tax rate per product or per category
- [ ] Multi-rate support (e.g., 0%, 7%, 10% on different product types)
- [ ] Tax breakdown on receipt and in order records

### oz-lua — Scripting Runtime
- [ ] Embed `rlua` Lua VM in `oz-lua`
- [ ] Expose `apply_discount()`, `calc_tax()`, `validate_order()` to Lua
- [ ] Merchant Lua scripts loaded from `scripts/` at runtime
- [ ] Lua sandbox: no filesystem or network access from scripts

### Hardware Expansion
- [ ] Bluetooth barcode scanner driver
- [ ] Cash drawer trigger (via receipt printer GPIO)
- [ ] Customer display driver (secondary screen, shows cart total)

### UI / UX — Staff, Transaction & Management Screens

**Staff & Auth**
- [ ] **Staff Login screen** — full-screen PIN pad (4–6 digit), staff avatar, name display
- [ ] **Role badge** — visible on all screens showing active user + role (Cashier / Manager / Owner)
- [ ] **Staff Management UI** — list of staff members, add/edit/deactivate, role assignment
- [ ] **Permission denied screen** — friendly message when cashier hits a manager-only action

**Transaction Screens**
- [ ] **Void Sale UI** — order lookup, void confirmation modal with reason picker
- [ ] **Refund / Return UI** — select items to refund (partial or full), amount preview, confirm
- [ ] **Hold Order UI** — name/tag a held order, resume from holds list
- [ ] **Split Bill UI** — drag items to split, assign amounts per person / payment method
- [ ] **Order History UI** — searchable list with filters (date, status, cashier), tap to view detail
- [ ] **Order Detail UI** — full receipt view with void/refund actions for managers

**Shift & Cash**
- [ ] **Open Shift screen** — enter opening cash balance, confirm
- [ ] **Close Shift screen** — count cash by denomination, compare to expected, print EOD summary
- [ ] **EOD Report screen** — sales total, cash, card, voids, discounts breakdown

**Business Rule UI**
- [ ] **Discount UI** — apply % or fixed discount to whole cart or individual line items
- [ ] **Tax Configuration UI** — inclusive/exclusive toggle, rate per category table
- [ ] **Customer Management UI** — search customers, view purchase history, link to order
- [ ] **Inventory Adjustment UI** — manual stock-in / stock-out with reason field

### Acceptance Criteria
- [ ] Void and refund flows update stock and produce audit entries
- [ ] Shift open/close persists cash reconciliation report
- [ ] A Lua discount script applies correctly at checkout
- [ ] Audit log entries are write-once (no UPDATE/DELETE allowed on `audit_log`)
- [ ] RBAC: cashier cannot access manager-only screens
- [ ] PIN login screen renders correctly on all target platforms

---

## Phase 4 — Scaling
> **Goal:** Multi-store, multi-terminal, cloud sync, payment gateways, and mobile builds.

### Cloud Sync (Optional On-Feature)
- [ ] SQLite outbox pattern (`outbox` table in `oz-core`)
- [ ] Background sync daemon: outbox → PostgreSQL (via `tokio-postgres`)
- [ ] Conflict resolution strategy (last-write-wins with timestamp)
- [ ] Cloud DB add-on: AWS RDS or Azure Database for PostgreSQL
- [ ] Redis cache: product look-ups, pricing rules, inventory pub/sub

### Multi-Store & Multi-Terminal
- [ ] Store entity: each store has its own settings + feature flags
- [ ] Multi-store management UI (owner view across all locations)
- [ ] Multi-terminal: terminals in the same store share inventory via cloud sync
- [ ] Per-terminal feature overrides (e.g., terminal A has KDS, terminal B does not)

### oz-payment
- [ ] `PaymentProcessor` trait definition
- [ ] Mock payment processor (for testing and offline demo)
- [ ] Stripe integration (card present + card not present)
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
- [ ] ARIA labels on all interactive elements
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

*Last updated: 2026-06-28.*
