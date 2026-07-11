//! Re-exports every `#[tauri::command]` module.
//!
//! Adding a new feature module:
//! 1. Create `commands/<feature>.rs` with at least one `#[tauri::command] async fn`.
//! 2. Add `pub mod <feature>;` here.
//! 3. Add the command(s) to the `invoke_handler!` macro in `lib.rs`.

pub mod audit;
pub mod auth;
pub mod authz;
pub mod branding;
pub mod bundles;
pub mod categories;
pub mod currencies;
pub mod customers;
pub mod data;
pub mod exchange_rates;
pub mod features;
pub mod gift_cards;
pub mod hardware;
pub mod health;
pub mod history;
pub mod inventory_counts;
pub mod kds;
pub mod loyalty;
pub mod offline;
pub mod plugins;
pub mod pos;
pub mod product_variants;
pub mod products;
pub mod promotions;
pub mod purchasing;
pub mod refunds;
pub mod reports;
pub mod sales;
pub mod scale;
pub mod settings;
pub mod setup;
pub mod shifts;
pub mod staff;
pub mod stock_transfers;
pub mod store_profiles;
pub mod sync;
pub mod tables;
pub mod tax;
pub mod terminals;
pub mod void;
pub mod workspaces;
