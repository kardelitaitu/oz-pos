# TODO — OZ-POS v0.0.4

## Rust Codebase Documentation Audit (1,400+ missing docs)

### Phase 1: Core Crates (~89 total)
  - `[x]` **`crates/oz-core/`** — Domain model & data access (was "foundation", ~15 doc + ~81 missing)
  - `[x]` Top-level core types: `lib.rs`, `error.rs`, `money.rs`, `migrations.rs`
  - `[x]` Auth & identity: `auth.rs`, `user.rs`, `user_preferences.rs`, `session.rs`, `subscription.rs`, `license_verification.rs`
  - `[x]` Products & inventory: `product.rs`, `product_variant.rs`, `product_bundle.rs`, `category.rs`, `sku.rs`, `inventory.rs`, `stock_count.rs`, `stock_transfer.rs`
  - `[x]` Sales & payments: `sale.rs`, `cart.rs`, `refund.rs`, `promotion.rs`, `gift_card.rs`, `payment.rs`, `exchange_rate.rs`
  - `[x]` Operations: `shift.rs`, `cash_payout.rs`, `table.rs`, `kds.rs`, `recipe.rs`, `purchase_order.rs`, `supplier.rs`
  - `[x]` Config & infrastructure: `settings.rs`, `store_profile.rs`, `terminal.rs`, `terminal_profile.rs`, `terminal_override.rs`, `features.rs`, `audit.rs`, `cache.rs`, `events.rs`, `offline.rs`, `ozpkg.rs`
  - `[x]` `db/` data access layer (~31 modules: sales, products, payments, inventory, settings, staff, customers, loyalty, promotions, etc.)
  - `[x]` Sync subsystem (was "platform-sync"): `sync/` (`mod.rs`, `lan_discovery.rs`) + `sync_client.rs`
- `[x]` **`crates/oz-api/`** — REST API crate (~24 warnings) *(no `missing_docs` lint; clippy clean)*
  - `[x]` Top-level: `lib.rs`, `auth.rs`
  - `[x]` API route handlers: `routes/` (`mod.rs`, `health.rs`, `tokens.rs`, `categories.rs`, `products.rs`, `sales.rs`)
- `[x]` **`crates/oz-plugin/`** — Plugin system (~46 warnings) *(no `missing_docs` lint; clippy clean)*
  - `[x]` All modules: `lib.rs`, `manifest.rs`, `manager.rs`, `loader.rs`, `package.rs`, `db.rs`, `error.rs`

### Phase 2: Tablet App (~501 warnings)
- `[x]` **`apps/tablet-client/src/`** — Tauri tablet commands *(no `missing_docs` lint; `cargo clippy` clean)*
  - `[x]` Top-level: `lib.rs`, `error.rs`, `state.rs`
  - `[x]` Auth & config commands: `auth.rs`, `authz.rs`, `branding.rs`, `features.rs`, `settings.rs`, `setup.rs`, `currencies.rs`, `exchange_rates.rs`
  - `[x]` Product & inventory commands: `products.rs`, `product_variants.rs`, `categories.rs`, `bundles.rs`, `inventory_counts.rs`, `stock_transfers.rs`
  - `[x]` Sales & payment commands: `pos.rs`, `sales.rs`, `refunds.rs`, `void.rs`, `gift_cards.rs`, `loyalty.rs`, `promotions.rs`, `purchasing.rs`
  - `[x]` Operations commands: `shifts.rs`, `staff.rs`, `tables.rs`, `kds.rs`, `history.rs`, `reports.rs`, `audit.rs`
  - `[x]` Hardware & infrastructure: `hardware.rs`, `scale.rs`, `health.rs`, `offline.rs`, `sync.rs`, `tax.rs`, `terminals.rs`, `customers.rs`, `data.rs`, `license.rs`, `plugins.rs`

### Phase 3: Desktop App (~811 warnings)
- `[x]` **`apps/desktop-client/src/`** — Tauri desktop commands *(no `missing_docs` lint; `cargo clippy` clean)*
  - `[x]` Top-level: `lib.rs`, `error.rs`, `state.rs`, `lan_server.rs`
  - `[x]` Auth & config commands: `auth.rs`, `authz.rs`, `branding.rs`, `features.rs`, `settings.rs`, `setup.rs`, `currencies.rs`, `exchange_rates.rs`
  - `[x]` Product & inventory commands: `products.rs`, `product_variants.rs`, `categories.rs`, `bundles.rs`, `inventory_counts.rs`, `stock_transfers.rs`
  - `[x]` Sales & payment commands: `pos.rs`, `sales.rs`, `refunds.rs`, `void.rs`, `gift_cards.rs`, `loyalty.rs`, `promotions.rs`, `purchasing.rs`
  - `[x]` Operations commands: `shifts.rs`, `staff.rs`, `tables.rs`, `kds.rs`, `history.rs`, `reports.rs`, `audit.rs`
  - `[x]` Hardware & infrastructure: `hardware.rs`, `scale.rs`, `health.rs`, `offline.rs`, `sync.rs`, `tax.rs`, `terminals.rs`, `customers.rs`, `data.rs`, `license.rs`, `plugins.rs`, `workspaces.rs`

---
## 1. Centralized Branding & Assets Architecture (`assets/`)

### Objective
Create a unified `assets/branding/` directory to store master default branding files (`OZ-POS`), generate platform-specific icons (Web, PWA, Tauri Desktop/Tablet, Thermal Receipts), and provide a clean, zero-code whitelabeling mechanism for enterprise tenants.

### Directory Structure & Whitelabel Layout
```
assets/
└── branding/
    ├── default/               # Default OZ-POS brand identity
    │   ├── manifest.json      # Brand tokens (name, HSL colors, font, logo mappings)
    │   ├── vector/            # Master scalable vector graphics (SVGs)
    │   ├── web/               # Web favicons & PWA icons
    │   ├── desktop/           # Tauri desktop app icons (ICO, ICNS, PNG)
    │   └── hardware/          # Thermal receipt bitmaps & invoice logos
    └── whitelabel/            # Tenant / partner override templates
        └── example-tenant/    # Example structure for custom whitelabel builds
```

---

## 2. Comprehensive Inventory of Required Assets & Formats/Sizes

### A. Vector / Master Logos (`assets/branding/default/vector/`)
Primary resolution-independent assets used across UI screens (`BrandContext`), headers, login screens, and PDF generators:
- `logo-full.svg` — Primary horizontal logo (wordmark + mark) for light UI themes.
- `logo-full-dark.svg` — Primary horizontal logo optimized for dark UI themes.
- `logo-mark.svg` — Compact square mark (no wordmark) for collapsed sidebars, loading spinners, and small headers.
- `logo-monochrome.svg` — High-contrast single-color vector for e-paper displays and document stamp vectors.

### B. Web Favicons & PWA Manifest (`assets/branding/default/web/` → `ui/public/`)
Standardized assets required by `index.html` and PWA web app manifests:
- `favicon.ico` — Multi-resolution ICO bundle (`16x16`, `32x32`, `48x48`) for legacy browsers and tab icons.
- `favicon.svg` — Modern scalable vector favicon for high-DPI browser tabs.
- `apple-touch-icon.png` — `180x180 px` transparent/solid PNG for iOS/iPadOS home screen shortcuts.
- `icon-192.png` — `192x192 px` transparent PNG for Android / PWA standard manifest icon.
- `icon-512.png` — `512x512 px` transparent PNG for PWA splash screens and app store installation.
- `icon-maskable-512.png` — `512x512 px` Adaptive Icon with safe-zone padding (for modern squircle/circle cutouts on Android/PWA).

### C. Tauri Desktop & Tablet App Icons (`assets/branding/default/desktop/` → `apps/*/icons/`)
Compiled from a master `1024x1024 px` PNG (`icon.png`) for native OS application packaging:
- `icon.ico` — Multi-resolution Windows Desktop icon (`16x16` up to `256x256 px`) used for `.exe` application icon, taskbar, and NSIS installer.
- `icon.icns` — Apple ICNS icon bundle (`32x32` up to `1024x1024 px` Retina) for macOS `.app` bundle and Dock.
- `icon.png` / `32x32.png` / `128x128.png` / `256x256.png` — Standard PNG sets for Linux `.desktop` entries, AppImage, and `.deb` packaging.
- `tray-icon.png` / `tray-icon.ico` — Compact `32x32 px` silhouette/monochrome icon specifically designed for OS system tray and menu bar indicator.

### D. Hardware & Receipt Assets (`assets/branding/default/hardware/`)
Specialized assets optimized for embedded hardware and printing (`oz-hal` / `oz-api`):
- `receipt-logo-58mm.png` / `.bin` — `384x100 px` 1-bit monochrome raster bitmap for 58mm thermal receipt printers (direct ESC/POS printing without runtime dithering/scaling artifacts).
- `receipt-logo-80mm.png` / `.bin` — `576x150 px` 1-bit monochrome raster bitmap for 80mm thermal receipt printers.
- `invoice-watermark.png` — Optional subtle watermark or stamp for backend PDF invoice generation (`oz-api`).

---

## 3. Whitelabel Implementation Plan

### A. Brand Configuration Manifest (`manifest.json`)
Every brand folder (`default/` or `whitelabel/<tenant>/`) must include a `manifest.json`:
```json
{
  "brandId": "default",
  "appName": "OZ-POS",
  "companyName": "OZ POS Inc.",
  "themeTokens": {
    "primaryHsl": "210, 80%, 55%",
    "accentHsl": "160, 75%, 45%",
    "fontFamily": "Inter, sans-serif"
  },
  "assets": {
    "logoFullLight": "vector/logo-full.svg",
    "logoFullDark": "vector/logo-full-dark.svg",
    "logoMark": "vector/logo-mark.svg",
    "receiptLogo80mm": "hardware/receipt-logo-80mm.png"
  }
}
```

### B. Build-Time Whitelabel Engine (`scripts/whitelabel.ps1` / `scripts/whitelabel.sh`)
Create a CLI tooling script to apply a brand across the codebase prior to compilation:
1. **Input:** `powershell -File scripts/whitelabel.ps1 -Brand "my-tenant"`
2. **Action:**
   - Reads `assets/branding/<tenant>/manifest.json`.
   - Copies web icons to `ui/public/` and updates `ui/public/manifest.json`.
   - Copies desktop icons (`icon.ico`, `icon.icns`, `icon.png`) to `apps/desktop-client/icons/` and `apps/tablet-client/icons/`.
   - Modifies `tauri.conf.json` (`productName`, `identifier`, `bundle.icon`).
   - Injects tenant brand variables into `ui/src/features/design/brand-tokens.json` or CSS root definitions.

### C. Runtime Brand Injection (`BrandContext.tsx`)
Enhance React UI `<BrandContext>` to dynamically consume the active brand configuration (`manifest.json` or IPC settings query from backend SQLite), allowing live theme updates and customized header logos per station or tenant session.

---

## Next Action Items (`v0.0.4`)
- `[ ]` Scaffold `assets/branding/default/` directory and all placeholder subdirectories (`vector/`, `web/`, `desktop/`, `hardware/`).
- `[ ]` Create initial `manifest.json` for default `OZ-POS` brand identity.
- `[ ]` Generate baseline SVG vectors (`logo-full.svg`, `logo-mark.svg`, `logo-monochrome.svg`) and basic web/desktop icon sizes.
- `[ ]` Write `scripts/whitelabel.ps1` helper to automate brand injection across Vite and Tauri project files.

