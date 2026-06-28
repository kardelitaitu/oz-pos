# oz-core

Domain models, SQLite persistence, and migrations for OZ-POS. Every other crate builds on types defined here.

## Public modules (19)

| Module | Key types |
|--------|-----------|
| `audit` | `AuditEntry` — structured audit log |
| `auth` | `StaffSession`, token generation |
| `cart` | `Cart`, `CartLine` — in-memory sale state machine |
| `category` | `Category` — product categories |
| `customer` | `Customer` — customer records |
| `db` | `Store` — all CRUD methods (products, sales, customers, staff, tax_rates, audit, features, currencies, exchange_rates, held_carts, barcode lookup) |
| `error` | `CoreError` — `thiserror`-based, `#[non_exhaustive]` |
| `exchange_rate` | `ExchangeRateRow` — currency conversion rates |
| `features` | Feature flag types |
| `inventory` | Stock adjustment types |
| `migrations` | `run(&mut Connection)` — applies pending SQL from `migrations/` (001–013) |
| `money` | `Money(i64, Currency)`, `Currency` (ISO-4217 newtype) |
| `ozpkg` | Package metadata types |
| `product` | `Product`, `ProductDto` — SKU, price, barcode, stock, tax links |
| `sale` | `Sale`, `SaleLine`, `SaleStatus`, `SaleSummary` |
| `settings` | Key-value settings accessors (receipt config, store info, currency) |
| `sku` | `Sku` newtype — validated stock-keeping unit |
| `tax_rate` | `TaxRate` — name, rate in basis points, is_default |
| `user` | `User`, `Role` — staff identity and permissions |

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

> last audited 28-06-26 by docs-auditor
