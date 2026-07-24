# oz-core

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (3 noted findings, doc-staleness) · F1: "Public modules (42)" -> 55 pub mod in lib.rs (table lists ~46) · F2: migrations "001-047" -> crates/oz-core/migrations/ now ranges 001-100 (98 .sql files) · F3 (minor): Money/Currency are defined in foundation/src/money.rs and re-exported from oz-core (pub use money::{Currency, Money}), not defined in oz-core · verified accurate: #![deny(unsafe_code)] + #![warn(missing_docs)] present in lib.rs, Money-i64 policy, Store-transaction convention -->

Domain models, SQLite persistence, and migrations for OZ-POS. Every other crate builds on types defined here.

## Public modules (42)

| Module | Key types |
|--------|-----------|
| `audit` | `AuditEntry` — structured audit log |
| `auth` | `StaffSession`, token generation |
| `cache` | In-memory cache helpers |
| `cart` | `Cart`, `CartLine` — in-memory sale state machine |
| `cash_payout` | Cash payout types |
| `category` | `Category` — product categories |
| `customer` | `Customer` — customer records |
| `db` | `Store` — all CRUD methods (products, sales, customers, staff, tax_rates, audit, features, currencies, exchange_rates, held_carts, barcode lookup) |
| `error` | `CoreError` — `thiserror`-based, `#[non_exhaustive]` |
| `events` | Domain event types |
| `exchange_rate` | `ExchangeRateRow` — currency conversion rates |
| `features` | Feature flag types |
| `gift_card` | Gift card types |
| `inventory` | Stock adjustment types |
| `kds` | Kitchen Display System types |
| `loyalty` | Loyalty points types |
| `migrations` | `run(&mut Connection)` — applies pending SQL from `migrations/` (001–047) |
| `money` | `Money(i64, Currency)`, `Currency` (ISO-4217 newtype) |
| `offline` | Offline queue types |
| `ozpkg` | Package metadata types |
| `payment` | Payment transaction types |
| `product` | `Product`, `ProductDto` — SKU, price, barcode, stock, tax links |
| `product_bundle` | Product bundle types |
| `product_variant` | Product variant types |
| `promotion` | Promotion types |
| `purchase_order` | Purchase order types |
| `refund` | Refund types |
| `sale` | `Sale`, `SaleLine`, `SaleStatus`, `SaleSummary` |
| `settings` | Key-value settings accessors (receipt config, store info, currency) |
| `shift` | Staff shift types |
| `sku` | `Sku` newtype — validated stock-keeping unit |
| `stock_count` | Stock count types |
| `stock_transfer` | Stock transfer types |
| `store_profile` | Store profile types |
| `supplier` | Supplier types |
| `sync_client` | Sync client types |
| `table` | Restaurant table types |
| `tax_rate` | `TaxRate` — name, rate in basis points, is_default |
| `terminal` | POS terminal registration types |
| `terminal_override` | Terminal feature override types |
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

Key methods: `create_product`, `list_products`, `update_product`, `delete_product`, `lookup_product_with_details_by_barcode`, `list_sales`, `get_sale`, `create_sale`, `complete_sale`, `hold_cart`, `list_held_carts`, `get_held_cart`, `delete_held_cart`, `set_cart_discount`, `export_daily_summary`, `export_sales_by_hour`, `export_eod_report`, staff CRUD, customer CRUD, category CRUD, tax rate CRUD, feature flags, currencies, exchange rates, audit log.

## Conventions

- Money is always `i64` minor units — never `f32`/`f64`.
- `#![deny(unsafe_code)]` and `#![warn(missing_docs)]`.
- All public items have `///` docs.

> last audited 07-07-26 by docs-auditor
