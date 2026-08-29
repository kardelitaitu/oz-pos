# oz-core

<!-- Audit stamp: 2026-08-30 · docs-auditor · status: ACCURATE (3 findings repaired) · F1: "Public modules (42)" -> 56 pub mod in lib.rs (table now complete) · F2: migrations "001-047" -> date-prefixed consolidated (20260813_init.sql etc.; 131 sequential migrations consolidated 2026-08-14) · F3: Money/Currency are defined in foundation/src/money.rs and re-exported from oz-core (pub use money::{Currency, Money}) -- already noted, now confirmed · verified accurate: #![deny(unsafe_code)] + #![warn(missing_docs)] present in lib.rs, Money-i64 policy, Store-transaction convention -->

Domain models, SQLite persistence, and migrations for OZ-POS. Every other crate builds on types defined here.

## Public modules (56)

| Module | Key types |
|--------|-----------|
| `audit` | `AuditEntry` — structured audit log |
| `auth` | `StaffSession`, token generation |
| `cache` | In-memory cache helpers |
| `cart` | `Cart`, `CartLine` — in-memory sale state machine |
| `cash_payout` | Cash payout types |
| `category` | `Category` — product categories |
| `config_validator` | Config validation helpers |
| `crypto` | Cryptographic helpers |
| `customer` | `Customer` — customer records |
| `db` | `Store` — all CRUD methods (products, sales, customers, staff, tax_rates, audit, features, currencies, exchange_rates, held_carts, barcode lookup) |
| `error` | `CoreError` — `thiserror`-based, `#[non_exhaustive]` |
| `events` | Domain event types |
| `export` | Export helpers |
| `features` | Feature flag types |
| `gift_card` | Gift card types |
| `inventory` | Stock adjustment types |
| `inventory_transaction` | Inventory transaction types |
| `kds` | Kitchen Display System types |
| `license_verification` | License verification types |
| `location_resolver` | Location resolution |
| `loyalty` | Loyalty points types |
| `migrations` | `run(&mut Connection)` — applies pending SQL from `migrations/` (date-prefixed, consolidated from 131 sequential migrations 2026-08-14) |
| `money` | `Money(i64, Currency)`, `Currency` (ISO-4217 newtype) |
| `offline` | Offline queue types |
| `ozpkg` | Package metadata types |
| `payment` | Payment transaction types |
| `popularity` | Product popularity index |
| `product` | `Product`, `ProductDto` — SKU, price, barcode, stock, tax links |
| `product_bundle` | Product bundle types |
| `product_variant` | Product variant types |
| `promotion` | Promotion types |
| `purchase_order` | Purchase order types |
| `rate_limiter` | Rate limiter |
| `recipe` | Recipe types |
| `refund` | Refund types |
| `sale` | `Sale`, `SaleLine`, `SaleStatus`, `SaleSummary` |
| `sale_deduction` | Sale deduction logic |
| `session` | Session types |
| `settings` | Key-value settings accessors (receipt config, store info, currency) |
| `shift` | Staff shift types |
| `sku` | `Sku` newtype — validated stock-keeping unit |
| `stock_count` | Stock count types |
| `stock_transfer` | Stock transfer types |
| `store_profile` | Store profile types |
| `subscription` | Subscription types |
| `supplier` | Supplier types |
| `sync` | Sync types |
| `sync_client` | Sync client types |
| `table` | Restaurant table types |
| `tax_rate` | `TaxRate` — name, rate in basis points, is_default |
| `terminal` | POS terminal registration types |
| `terminal_override` | Terminal feature override types |
| `terminal_profile` | Terminal profile types |
| `topology` | Topology types |
| `user` | `User`, `Role` — staff identity and permissions |
| `user_preferences` | User preference types |

## Money

```rust
use oz_core::{Money, Currency};

let usd = Currency::from_str("USD").unwrap();
let price = Money::from_major(12, usd);
let total = price.checked_add(Money::from_major(5, usd)).unwrap();
assert_eq!(total.minor_units, 1700);
```

## Store (SQLite)

All DB access goes through `Store` methods in `db.rs`. Every write runs inside a `rusqlite` transaction.

Key methods: `create_product`, `list_products`, `update_product`, `delete_product`, `lookup_product_with_details_by_barcode`, `list_sales`, `get_sale`, `create_sale`, `complete_sale_deduction`, `complete_sale_with_resolved_shortfalls`, `hold_cart`, `list_held_carts`, `get_held_cart`, `delete_held_cart`, `set_cart_discount`, `export_daily_summary`, `export_sales_by_hour`, staff CRUD, customer CRUD, category CRUD, tax rate CRUD, feature flags, currencies, exchange rates (`list_exchange_rates`, `create_exchange_rate`, `upsert_exchange_rate`), audit log.

## Conventions

- Money is always `i64` minor units — never `f32`/`f64`.
- `#![deny(unsafe_code)]` in `lib.rs`; `missing_docs` is warned via
  `[lints] workspace = true`, inherited from the root `[workspace.lints]`.
- All public items have `///` docs.

> last audited 30-08-26 by docs-auditor
