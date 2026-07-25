<!-- Audit stamp: 2026-07-25 · Buffy-Agent · status: ACCURATE (0 findings) · modules/currency/src/lib.rs exists with CurrencyModule implementing the Module trait; modules/currency/manifest.json present and matches spec schema; CurrencyRepository with exchange-rate, currency-list, and currency-format methods fully migrated from oz-core (R2 Phase 1–6 complete); Platform error variant added to CurrencyError for settings delegation -->

# Currency/Exchange Module

**Status:** Production (R2 Complete)

## Overview

The Currency/Exchange module manages currencies and exchange rates. It provides the ISO-4217 currency table, default currency configuration, exchange rate CRUD, and currency-format settings (symbol position, decimal/thousands separators) for multi-currency transactions.

## Module Info

| Field        | Value                        |
|--------------|------------------------------|
| ID           | `currency`                   |
| Version      | `1.0.0`                      |
| Dependencies | `[]`                         |
| Permissions  | `currency:view`, `currency:edit` |

## Lifecycle

The module implements `foundation::contracts::Module` and follows the standard lifecycle:

1. **`on_load`** — Validates configuration
2. **`on_start`** — Prepares for currency operations
3. **`on_stop`** — Cleans up resources

## Registration

Registered with the kernel during application setup:

```rust
use modules_currency::CurrencyModule;
use platform_kernel::Kernel;

let mut kernel = Kernel::new();
kernel.register(Box::new(CurrencyModule::new()))?;
kernel.load_all()?;
kernel.start_all()?;
```

## Repository

`CurrencyRepository` provides typed database access for all currency operations:

```rust
pub struct CurrencyRepository<'a> {
    conn: &'a Connection,
}
```

### Exchange-Rate Methods
- `list_exchange_rates()` — Returns all exchange rates
- `create_exchange_rate()` — Insert a new rate
- `upsert_exchange_rate()` — Insert or update by currency pair
- `delete_exchange_rate()` — Remove by ID

### Currency-List Methods
- `list_currencies()` — Returns all ISO-4217 currencies (`Vec<CurrencyDto>`)

### Currency-Format Settings Methods
- `get_default_currency()` / `set_default_currency(code)`
- `get_currency_format()` / `set_currency_format(fmt)` — `"symbol"` or `"code"`
- `get_currency_symbol_position()` / `set_currency_symbol_position(pos)` — `"prefix"` or `"suffix"`
- `get_currency_decimal_separator()` / `set_currency_decimal_separator(sep)` — `"dot"` or `"comma"`
- `get_currency_thousands_separator()` / `set_currency_thousands_separator(sep)` — `"comma"`, `"dot"`, `"space"`, or `"none"`

All settings methods delegate to `platform_core::settings::Settings` and convert errors via the `Platform` variant on `CurrencyError`.

### Error Handling

```rust
pub enum CurrencyError {
    #[from] Db(rusqlite::Error),
    Validation { field: String, message: String },
    NotFound { entity: &'static str, id: String },
    #[from] Platform(platform_core::PlatformError),
}
```

Conversion from `CurrencyError` to `CoreError` is provided via `From<CurrencyError> for CoreError`.

### Deprecated oz-core Wrappers

The original 15 delegating Store methods in `oz-core` are marked `#[deprecated]` and direct callers to use `CurrencyRepository` directly. Tests retain `#[allow(deprecated)]` for backward compatibility.

## Manifest

```json
{
  "id": "currency",
  "name": "Currency/Exchange",
  "version": "1.0.0",
  "dependencies": [],
  "permissions": ["currency:view", "currency:edit"]
}
```
