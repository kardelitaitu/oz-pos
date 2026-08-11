# ADR #36: Retail POS Product Attributes — Cost, Brand, Rack, Notes + Configurable Columns

Date: 2026-08-11

Status: Accepted

## Context

The retail POS product grid (`ui/src/features/retail/RetailProductGrid.tsx`) is
already a table — columns SKU | Stock | Name | Price | Action, with sorting
(`SortField = 'sku' | 'name' | 'stock' | 'price'`), pagination, category chips,
search, and a SKU lookup bar. It is driven by `RetailPosScreen.tsx`, which loads
the real catalog through `loadCatalog(token)` →
`listProductsScoped(sessionToken)` (`ui/src/utils/catalog-cache.ts`, PERF-08).

Retailers need more merchandising and costing attributes than the current four
columns expose:

1. **Cost (HPP / harga pokok penjualan)** — the purchase price of the product.
   The column already exists in the schema (`products.cost_minor`, migration
   `054_product_cost.sql`) but is **not surfaced** in the core `Product` reads,
   the DTOs, the Tauri commands, or any UI. It is only consumed by the
   menu-engineering report (`crates/oz-reporting/src/menu_engineering.rs`).
2. **Brand** — no column exists anywhere.
3. **Rack position code** — no column exists; staff currently cannot see where
   a product lives in the store. The retail POS **Stock Inquiry** sub-view
   (`RetailSubViews.tsx` → `ProductLookupScreen.tsx`) is the natural surface
   for it.
4. **Notes** — no column exists (`products` has no description/notes field).

Separately, the operator wants a **configurable column set** on the retail
product grid: a column picker with per-user persistence (the same mechanism the
KDS multi-layout system uses via `user_preferences`, migration
`038_user_preferences.sql`).

Two gaps discovered while scoping must be addressed or the feature cannot work
end to end:

- **Retail POS product edits do not persist.** `RetailPosScreen.tsx`
  `handleSaveProductEdit` / `handleSaveNewProduct` mutate local React state and
  call `invalidateCatalog` only — nothing is written to the backend. A catalog
  reload discards the change. Entering a cost in the retail POS therefore
  requires wiring the retail add/edit flows to the store-scoped product
  commands (`create_product_scoped` / `update_product_scoped`, ADR #7).
- **Stock is not a total.** The product queries LEFT JOIN the legacy
  `inventory` table (single row per product). Since ADR-19, the canonical
  per-location source is `stock_summary` (composite PK `(item_id, location_id)`,
  migration 089). "Total unit of the product" = `SUM(qty)` across locations.

The **back-office Product Management screen**
(`ui/src/features/products/ProductManagementScreen.tsx`) is explicitly out of
scope for this change — the product attributes work happens in the retail POS
app.

## Decision

### D1 — Product attribute model (schema)

One new forward-only migration, `132_product_attributes.sql`, registered in the
compile-time `ALL` array in `crates/oz-core/src/migrations.rs` (unique prefix
after 131; the registry↔filesystem parity and monotonic-prefix tests enforce
this):

```sql
ALTER TABLE products ADD COLUMN brand TEXT;              -- free text, nullable
ALTER TABLE products ADD COLUMN rack_location TEXT;      -- e.g. "A-01-03", nullable
ALTER TABLE products ADD COLUMN notes TEXT;              -- free text, nullable
ALTER TABLE products ADD COLUMN unit TEXT;               -- UOM: 'pcs', 'kg', 'box', 'dozen', ... nullable
ALTER TABLE products ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;
ALTER TABLE products ADD COLUMN default_supplier_id TEXT REFERENCES suppliers(id);
```

`cost_minor` already exists (migration 054, `INTEGER NOT NULL DEFAULT 0`) and is
**not re-migrated** — it becomes first-class through the read/write plumbing
below. Simple `ADD COLUMN` is safe: migration 117 already rebuilt `products`
with the full column set, and new nullable columns do not affect the upgrade
path (verified by the existing fresh-install-vs-upgrade fingerprint test). No
`cost_updated_at` column: cost is not auto-derived, so the existing
`price_updated_at` semantics are untouched.

Semantics of the three additional attributes (kept from the Tier-1 review):
`unit` is free text for now (normalized later if reporting demands it, same
pattern as brand); `is_active` matches the existing
`product_variants.is_active` semantics — hide/retire a product without deleting
it, preserving sales history; `default_supplier_id` is a nullable FK to
`suppliers` (migration 046) — the preferred supplier used to prefill purchase
orders and future reorder/analytics.

### D2 — Cost is local-only (data residency)

`cost_minor` is **excluded from the cloud sync snapshot and the cloud server**.
The sync surface (`platform/sync/src/transport.rs` `SnapshotProduct`,
`crates/oz-core/src/sync_client.rs` product upsert/import,
`platform/sync/src/pg_transport.rs` / `pg_daemon.rs`,
`apps/cloud-server/src/sync_api.rs`, and
`platform/sync/tests/pg_integration.rs`) gains the catalog fields `brand`,
`rack_location`, `notes`, `unit`, and `is_active` — each `#[serde(default)]`
for backward compatibility, with `SNAPSHOT_SCHEMA_VERSION` unchanged (additive
fields). **Not synced:** `cost_minor`, `default_supplier_id` (suppliers are
local purchasing data — the sync snapshot carries products/tax_rates/users
only), and the popularity fields (ADR #37 D4). This mirrors the ADR #35 D6
data-residency precedent: cost price stays on the device, per the store-first
model (ADR #4). The cost data remains available to the future analytics tool
because it is read from the local store DB, not the cloud.

### D3 — Backend plumbing

- **Core model** (`crates/oz-core/src/db/mod.rs` `row_to_product`, the
  `Product` struct): add `cost_minor: i64`, `brand`, `rack_location`, `notes`,
  `unit` (Option<String>), `is_active: bool`, `default_supplier_id`
  (Option<String>).
- **`crates/oz-core/src/db/products.rs`**: add the six new columns to every
  `SELECT` (list/get/lookup/update re-read), to `create_product` and
  `create_product_if_absent_in_tx` INSERTs, and to `update_product` UPDATEs
  (brand/rack/notes/unit/default_supplier_id clearable via `NULL`; cost always
  a non-negative i64; `is_active` a boolean). The product cache serializes
  `Product` — new fields flow through the existing cache path.
- **Stock total (D1 "total unit"):** the enriched product queries compute
  stock as `SUM(stock_summary.qty)` across all locations for the product
  (`stock_summary` is the canonical per-location ledger since ADR-19),
  falling back to `0`. The legacy `inventory` LEFT JOIN is replaced. This
  applies to the retail-facing queries; the Product Management screen is
  untouched.
- **Foundation DTOs** (`foundation/src/dto.rs`): `CreateProductDto` gains
  `cost_minor: i64` (default 0), `brand`, `rack_location`, `notes`, `unit`
  (Option<String>), `is_active: bool` (default true), `default_supplier_id`
  (Option<String>). `UpdateProductDto` (PATCH semantics) gains
  `cost_minor: Option<i64>`, `is_active: Option<bool>`, and clearable
  `Option<Option<String>>` for brand/rack/notes/unit/default_supplier_id —
  `null` clears, absent is a no-op (the existing `deserialize_optional_field`
  pattern is reused).
- **Tauri commands** (`apps/desktop-client/src/commands/products.rs`,
  `apps/tablet-client/src/commands/products.rs`): list/get/create/update
  (scoped + unscoped) carry the four fields in both directions. Cost is not
  written by the sync path (D2).

### D4 — Retail POS grid: configurable columns

`RetailProductGrid.tsx` + `RetailPosScreen.tsx` gain a **column visibility
toggle** (header control, localized, keyboard-accessible) over this set, in the
operator's preferred order:

**SKU | Barcode | Category | Brand | Name | Rack | Stock | Price | Notes | Action**

- New columns: **Barcode, Category, Brand, Rack, Notes** (SKU, Name, Stock,
  Price, Action exist today; **Type is omitted** — the retail grid only ever
  lists `product_type === 'retail'`).
- **Cost is deliberately not a column.** It is an entry/override field in the
  Add/Edit modals only (D5) — never rendered in the grid.
- **Persistence:** the visibility set is stored per user via the existing
  `get_user_preferences_scoped` / `set_user_preferences_scoped` API
  (`ui/src/api/settings.ts`) under a `retail.visible_columns` key (JSON array),
  the KDS `useKdsPreferences` precedent. Absent/unknown keys fall back to the
  default set. No new backend surface.
- Columns render `—` when the value is empty.
- A **hide-inactive filter** in the grid toolbar hides `is_active = 0`
  products by default (status is managed in the Edit modal, D5).
- The **Stock column shows the total across locations** (D1). Sort remains on
  the existing four fields; adding brand/cost sort is a cheap follow-up, not a
  requirement.

### D5 — Cost editing surfaces (retail POS only)

Cost (and the new attributes) are entered manually in three places, per the
product owner:

1. **Add Product modal** (`ui/src/features/retail/AddProductModal.tsx`) — Cost,
   Brand, Rack, Notes, **Unit (UOM)**, **Status (active/inactive)**, and
   **Default supplier** fields.
2. **Edit Product modal** (`ui/src/features/retail/EditProductModal.tsx`) —
   the same fields (Cost, Brand, Rack, Notes, Unit, Status, Default supplier).
   When the **stock quantity is increased**, an optional "Cost (override)"
   input appears — the "when we add stock we can input again if we want to
   override" flow, inside the retail POS app.
3. Manager can edit cost at any time via the Edit modal (role gating of the
   *cost field* is a follow-up, see D7).

Cost is never rendered as a grid column (D4) — it exists only in these edit
surfaces.

To make these edits real, `RetailPosScreen.tsx` `handleSaveNewProduct` /
`handleSaveProductEdit` are wired to `createProductScoped(sessionToken, …)` /
`updateProductScoped(sessionToken, …)` (all fields, including the new ones),
then `invalidateCatalog`. This closes the current local-only edit gap: the
retail POS becomes a complete catalog-management surface, and cost entry
persists across reloads.

### D6 — Stock Inquiry / Product Lookup shows Rack

`ProductLookupScreen.tsx` (the retail POS Stock Inquiry sub-view) is a card
grid (react-window). Each card gains a **Rack** line (and keeps stock status).
This is the approved "staff can see where the item lives" surface. Cost is not
shown here — per D4/D5 it is never rendered outside the Add/Edit modals.

### D7 — Explicitly deferred

- **PO-receive auto-cost** (updating `cost_minor` from PO line costs on
  receive) — deferred; cost is manual by decision.
- **Margin % / stock-value columns** — deferred; the data (price, cost, total
  stock) is stored and queryable, and will feed the analytics tool.
- **Role-gated cost access** — the cost field in the Add/Edit modals is
  reachable by any retail POS session for now; if staff/cashier access is
  granted later, gate cost editing on a permission key (ADR #35 pattern) —
  never UI-only.
- **Product Management screen columns** — untouched in this change.
- **Brand as a lookup table** — free text for now; a normalized `brands` table
  is a future decision if filtering/reporting demands it.
- **Rack as structured AIS/zone** — free text (`rack_location`) for now.

## Consequences

- Retailers can record purchase price, brand, rack position, and notes per
  product from the retail POS app, see them in a configurable grid, and rely on
  the data for the upcoming analytics tool.
- Cost is stored where it belongs (local store DB) and never leaves the device
  through sync; brand/rack/notes stay consistent across synced stores.
- The retail POS add/edit product flows become persistent — a correctness fix
  beyond the new fields, and the enabler for cost entry.
- The Stock column becomes location-aware (total across locations), matching
  the multi-location inventory model (ADR-19) instead of the legacy single-row
  `inventory` value.
- Existing databases upgrade cleanly (nullable ADD COLUMN; `cost_minor` already
  present; snapshot fields are `serde(default)`).

## Tradeoffs / risks

- **Cost reachable from the Edit modal by any retail POS session.** The row
  Edit button is available to whoever operates the retail POS, so the cost
  field in the modal is not manager-exclusive. Mitigation: cost is never a
  grid column (D4), and D7 records the permission-gating follow-up (hide or
  disable the cost field for non-manager roles) if a store requires it.
  Accepted for this release.
- **Free-text brand/rack** means no autocomplete or referential integrity.
  Accepted: normalized tables are deferred (D7).
- **Stock-total query change** touches every consumer of the enriched product
  queries (retail grid, Stock Inquiry, lookup). Risk is low (single shared
  query shape) but the change is verified by the existing product-query unit
  tests, updated for the SUM semantics.
- **Retail add/edit persistence** changes the contract of two modal flows that
  were intentionally local in demo mode; existing UI tests that assert
  local-state behavior are updated to the scoped-command contract.

## Verification

- Migration: registry/filesystem parity, monotonic prefix, fresh-vs-upgrade
  fingerprint tests stay green; new test asserts the six new columns exist and
  `cost_minor` is untouched.
- Core: product create/update round-trip tests assert cost/brand/rack/notes
  persist (including clearing brand/rack/notes via `null`); stock-total test
  seeds two locations and asserts the SUM.
- DTO: serde tests for the new `CreateProductDto` fields and the
  `UpdateProductDto` clear-vs-absent semantics.
- Commands: scoped create/update/list carry the new fields; denial/error tests
  unchanged.
- Sync: snapshot round-trip includes brand/rack/notes and **excludes**
  `cost_minor`; `pg_integration` schema updated accordingly.
- UI: `RetailPosScreen`/`RetailProductGrid` tests assert the new columns
  render (**and that no Cost column exists**), the toggle hides/shows and
  persists via `user_preferences` (mocked IPC), and the Stock column shows the
  total. Edit-modal tests assert cost entry and the stock-increase
  cost-override field. `ProductLookupScreen` card shows Rack.
- i18n: new FTL keys in both `ui/src/locales` bundles; bundle-parity and
  dedupe gates pass.
- Final single verification pass: `cargo fmt --all`, `cargo check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, targeted cargo
  tests, `npm run typecheck` / `npm run lint` / `npm run test` in `ui/`,
  i18n lint.
