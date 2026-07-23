---
name: rust-backend
description: Rust & database standards for the OZ-POS framework — Money struct, rusqlite transactions, thiserror/anyhow, clippy, doc comments. Use when adding or modifying Rust code in any `oz-*` crate.
---

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (3 noted findings, doc-staleness) · F1 (wrong path, recurring): line 134 says migrations live in migrations/<timestamp>_<name>.sql; no root migrations/ exists, actual crates/oz-core/migrations/NNN_*.sql (98 .sql files, e.g. 001_sales.sql) · F2 (wrong claim, recurring): line 231 says mocks gated by #[cfg(test)] or a mock feature; oz-hal/Cargo.toml has no [features] section at all (no mock feature; mocks not feature-gated — same drift as hal-drivers) · F3 (advisory): line 135 suggests r2d2_sqlite/deadpool-sqlite pooling; not present in workspace Cargo.toml · verified accurate: Money/Currency struct shape matches foundation/src/money.rs (minor_units:i64, currency:Currency, Currency(pub [u8;3])), i64-minor-units + thiserror conventions hold -->

# Rust Backend & Database Standards

The OZ-POS framework is built on Rust. This skill enforces the project's coding standards, especially around **money safety**, **database integrity**, and **error handling**.

---

## When to use

- Adding or modifying code in any `oz-*` crate (`oz-core`, `oz-hal`, `oz-lua`, `oz-security`, `oz-payment`, `oz-reporting`, `oz-logging`, `oz-cli`).
- Writing a new module, struct, or public function in Rust.
- Working with the `Money` struct, currency codes, or pricing.
- Writing or reviewing SQL migrations and `rusqlite` calls.
- Adding or changing error types.

---

## Golden rules (never break these)

| # | Rule | Why |
|---|------|-----|
| 1 | **Money is always `i64` minor units, never `f32`/`f64`.** | Floating point loses pennies. POS bugs in money = lost revenue or angry customers. |
| 2 | **All database writes happen inside a `rusqlite::Transaction`.** | Partial writes corrupt the local store and break offline-first guarantees. |
| 3 | **`thiserror` for library errors, `anyhow` for application errors.** | Library consumers need typed errors; top-level `main`/CLI uses `anyhow::Result`. |
| 4 | **All public items have `///` doc comments.** | The framework is meant to be consumed by third parties (Lua scripts, plugins, etc.). |
| 5 | **`cargo clippy -- -D warnings` must pass.** | Warnings are bugs-in-waiting. Treat them as errors. |

---

## The `Money` struct

Money is **always** stored and passed as integer minor units (e.g., cents for USD, sen for IDR, paise for INR). Never use floats anywhere in the money path.

```rust
//! Money is stored as integer minor units (e.g., cents) to avoid float
//! rounding. Pair with an ISO-4217 currency code for display.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the smallest currency unit (e.g., cents for USD).
    pub minor_units: i64,
    /// ISO-4217 currency code, e.g. "USD", "IDR", "EUR".
    pub currency: Currency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency(pub [u8; 3]); // ISO-4217 alpha-3

impl std::str::FromStr for Currency {
    type Err = InvalidCurrencyCode;
    fn from_str(s: &str) -> Result<Self, Self::Err> { /* … */ }
}

impl Money {
    /// Construct from a major-unit amount (e.g., dollars). Multiplies by
    /// the currency's exponent before storing.
    ///
    /// Returns `None` if the resulting minor-unit amount would overflow
    /// `i64` (e.g. `from_major(i64::MAX, KWD)` — 3 decimal places). Callers
    /// that hit this are doing something pathological, but returning
    /// `None` keeps the panic-free invariant intact.
    #[must_use]
    pub fn from_major(major: i64, currency: Currency) -> Option<Self> {
        let exp = currency.minor_unit_exponent();   // returns u32
        major.checked_mul(10_i64.pow(exp)).map(|minor_units| Self {
            minor_units, currency,
        })
    }

    #[must_use]
    pub fn zero(currency: Currency) -> Self {
        Self { minor_units: 0, currency }
    }

    /// Add two Money values. Returns `None` if the currencies differ
    /// (caller must convert first) or if the sum overflows `i64`.
    ///
    /// A currency mismatch is treated as a domain error rather than a
    /// panic so that the rule "never panic in library code" holds.
    /// If a specific call-site really cannot tolerate a mismatch, use
    /// `.expect("same-currency add")` at that point.
    #[must_use]
    pub fn checked_add(self, other: Money) -> Option<Money> {
        if self.currency != other.currency { return None; }
        self.minor_units.checked_add(other.minor_units).map(|v| Self {
            minor_units: v, currency: self.currency,
        })
    }
}
```

**Rules:**
- Never write `let total: f64 = 0.10 + 0.20;` — use `Money`.
- Never serialize `Money` to a float. `minor_units: i64` is the wire format.
- Currency conversion is a separate function that returns `Option<Money>` (lossy conversions fail).
- **`#[must_use]` on every `Money` constructor (`zero`, `from_major`, `checked_add`, …).** Silently dropping a freshly built `Money` is a common bug — `Money::zero(usd);` should be a compile error, not a no-op.
- Construct `Currency` from a string via `"USD".parse::<Currency>()` (the `FromStr` impl) rather than from raw bytes. The `FromStr` impl validates shape; `Currency(*b"USD")` does not.
- When displaying in the UI, the front-end calls `format_minor_units(money)` to render `"$1,234.56"`.

---

## Database access (rusqlite + transactions)

All writes go through a transaction. There are no exceptions. Use `Connection::unchecked_transaction()` or `Transaction::new()` for explicit control.

```rust
use rusqlite::{Connection, Transaction, params};

pub fn record_sale(
    conn: &mut Connection,
    sale: &Sale,
    lines: &[SaleLine],
) -> Result<(), CoreError> {
    let tx: Transaction = conn.transaction()?;
    insert_sale_header(&tx, sale)?;
    for line in lines {
        insert_sale_line(&tx, sale.id, line)?;
        decrement_stock(&tx, line.sku, line.qty)?;
    }
    tx.commit()?;  // <-- only on success
    Ok(())
}
```

**Rules:**
- A function that writes must take `&mut Connection` (or `&Transaction`) — never `&Connection`.
- Use `?` everywhere; let `tx.commit()` happen only on the happy path. A `?` before `commit()` triggers `Drop`, which rolls back automatically.
- Migrations live in `migrations/<timestamp>_<name>.sql` and are run by `oz-cli migrate`.
- Use `r2d2_sqlite` or `deadpool-sqlite` for connection pooling in the Tauri runtime.
- For read-only queries, you may use `&Connection` and skip the transaction.

---

## Error handling

### Library crates (`oz-core`, `oz-hal`, `oz-payment`, `oz-reporting`)

Use `thiserror` and define a domain error enum. Mark the enum `#[non_exhaustive]` so you can add variants without breaking semver.

```rust
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("money overflow: {left} {currency} + {right}")]
    MoneyOverflow { left: i64, right: i64, currency: String },

    #[error("currency mismatch: {0} vs {1}")]
    CurrencyMismatch(String, String),

    #[error("not found: {entity} {id}")]
    NotFound { entity: &'static str, id: String },
}
```

### Application layer (`oz-cli`, Tauri `main.rs`, scripts)

Use `anyhow` for ergonomic error propagation and context chaining.

```rust
use anyhow::{Context, Result};

fn run_migrations() -> Result<()> {
    let conn = open_db().context("opening SQLite database")?;
    let report = migrate(&conn, MIGRATIONS).context("running migrations")?;
    println!("Applied {} migrations in {:?}", report.count, report.elapsed);
    Ok(())
}
```

**Rules:**
- Never `unwrap()` or `expect()` in production code. Use `?` with context.
- Tests may use `unwrap()` and `expect()` freely.
- In `core` modules, never propagate `anyhow::Error` from a public function — always convert to your crate's typed error.

---

## Doc comments

Every public item gets `///`. Module-level docs use `//!` at the top of the file.

```rust
//! Domain types for the sales pipeline: carts, lines, totals, and the
//! state machine that advances a sale from `Open` to `Completed`.

/// A line item in an open cart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleLine {
    pub sku: Sku,
    pub qty: i64,
    pub unit_price: Money,
}
```

**Rules:**
- First sentence is a short summary. Tooltips and search use it.
- Include a `# Examples` section for non-trivial types.
- `# Errors` for fallible functions.
- `# Panics` for functions that can panic.
- Link to related types with `[`TypeName`]` — `rustdoc` will resolve them.

---

## Toolchain (run before commit)

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib
```

`AGENTS.md` makes `rustfmt` and `clippy -D warnings` mandatory. Don't commit code that fails either.

---

## Module layout conventions

- One public type per file when it's a major domain entity (`money.rs`, `currency.rs`, `cart.rs`).
- Re-export from `mod.rs` so external code can do `use oz_core::Money;`.
- Use `#[cfg(test)] mod tests { ... }` at the bottom of every file with testable logic.
- Mock implementations of traits live in `hal/src/drivers/mock.rs` and are gated by `#[cfg(test)]` or a `mock` feature.

---

## Common pitfalls

1. **Writing `total / 100.0`** to convert minor units to a display value. Use integer math: `total / 100` (or whatever the exponent is) and format the remainder separately. Better: let the front-end format.
2. **Constructing `Money` from a `String` user input** without parsing it explicitly as minor units. Always require callers to specify the unit.
3. **Using `Vec<Money>` to "batch" amounts** — sums can overflow `i64` at ~9.2 × 10¹⁸. For totals use `Option<Money>` and check `checked_add`.
4. **Forgetting to drop a `Transaction`** — if you return `Err(_)` before `commit()`, the `Drop` rolls back. Good. But if you `commit()` and then later return `Err(_)`, you've committed. Reorder so `commit()` is the last line.
5. **Reading and writing in the same connection from different threads** — `Connection` is `!Sync`. Use a pool, or move the connection into a `spawn_blocking` task.
6. **`String` for currency codes** — wrap in a `Currency` newtype so the type system enforces ISO-4217 shape.

---

## See also

- **[`tauri-ipc`](../tauri-ipc/SKILL.md)** — the Tauri command layer that exposes `oz-core` types (`Money`, `CartId`, `Sku`, …) to the front-end. Every new domain type you add here eventually crosses the IPC boundary; read `tauri-ipc` to see how it should be wrapped for JSON.
- **[`project-scaffold`](../project-scaffold/SKILL.md)** — the Cargo workspace, CI, and Git conventions that govern where this code lives and how it's released.
- **[`skill-drift-guard`](../skill-drift-guard/SKILL.md)** — run after a public-API change to confirm this skill still matches the code.

---

> last audited 19-07-26 by skill-drift-guard
