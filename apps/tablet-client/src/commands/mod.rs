//! Re-exports every `#[tauri::command]` module.
//!
//! Adding a new feature module:
//! 1. Create `commands/<feature>.rs` with at least one `#[tauri::command] async fn`.
//! 2. Add `pub mod <feature>;` here.
//! 3. Add the command(s) to the `invoke_handler!` macro in `lib.rs`.

pub mod audit;
pub mod auth;
pub mod categories;
pub mod customers;
pub mod currencies;
pub mod exchange_rates;
pub mod features;
pub mod hardware;
pub mod health;
pub mod history;
pub mod offline;
pub mod pos;
pub mod product_variants;
pub mod products;
pub mod refunds;
pub mod sales;
pub mod settings;
pub mod setup;
pub mod staff;
pub mod sync;
pub mod tax;
pub mod terminals;
pub mod void;
