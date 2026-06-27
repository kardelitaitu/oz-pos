# oz-core

Domain types for OZ-POS: money, currency, cart, sale, inventory. Every other crate in the workspace builds on the types defined here.

## Public API

- [`Money`](src/money.rs) — monetary amount in integer minor units (cents, sen, paise). Always paired with a `Currency`. Checked arithmetic via `Money::checked_add`.
- [`Currency`](src/money.rs) — ISO-4217 alpha-3 newtype. `Currency::from_str("USD")` validates the input.
- [`CoreError`](src/error.rs) — `thiserror`-based domain error. `#[non_exhaustive]` so new variants are non-breaking.

## Example

```rust
use oz_core::{Money, Currency};

let usd = Currency::from_str("USD").expect("valid ISO-4217");
let price = Money::from_major(12, usd);
let total = price.checked_add(Money::from_major(5, usd)).unwrap();
assert_eq!(total.minor_units, 1700);
```

## Conventions

- **Money is always `i64` minor units.** Never `f32`/`f64`.
- `#![deny(unsafe_code)]` and `#![warn(missing_docs)]` are on.
- All public items have `///` docs.

See the `rust-backend` skill under `.agents/skills/` for the full set of conventions.
