//! Re-exports every `#[tauri::command]` module.
//!
//! Adding a new feature module:
//! 1. Create `commands/<feature>.rs` with at least one `#[tauri::command]` async fn.
//! 2. Add `pub mod <feature>;` here.
//! 3. Add the command(s) to the `invoke_handler!` macro in `lib.rs`.

/// Audit log commands (view, filter, export).
pub mod audit;
/// Authentication commands (login, logout, refresh).
pub mod auth;
/// Authorization commands (roles, permissions).
pub mod authz;
/// Store branding commands.
pub mod branding;
/// Product bundle commands.
pub mod bundles;
/// Category CRUD commands.
pub mod categories;
/// Currency management commands.
pub mod currencies;
/// Customer CRUD commands.
pub mod customers;
/// Data export / import commands.
pub mod data;
/// Exchange-rate commands.
pub mod exchange_rates;
/// Feature-flag commands.
pub mod features;
/// Gift-card management commands.
pub mod gift_cards;
/// Hardware / peripheral commands.
pub mod hardware;
/// Health-check commands.
pub mod health;
/// Sales-history commands.
pub mod history;
/// Multi-location inventory, shifts, transactions, and thresholds commands.
pub mod inventory;
/// Inventory-count commands.
pub mod inventory_counts;
/// Kitchen Display System commands.
pub mod kds;
/// License commands.
pub mod license;
/// Loyalty / rewards commands.
pub mod loyalty;
/// Offline-mode commands.
pub mod offline;
/// Plugin management commands.
pub mod plugins;
/// Point-of-sale flow commands.
pub mod pos;
/// Product-variant commands.
pub mod product_variants;
/// Product CRUD commands.
pub mod products;
/// Promotion commands.
pub mod promotions;
/// Purchasing / purchase-order commands.
pub mod purchasing;
/// Refund commands.
pub mod refunds;
/// Reporting commands.
pub mod reports;
/// Sale / transaction commands.
pub mod sales;
/// Weight-scale commands.
pub mod scale;
/// Settings CRUD commands.
pub mod settings;
/// Initial-setup commands.
pub mod setup;
/// Shift management commands.
pub mod shifts;
/// Staff / employee commands.
pub mod staff;
/// Stock-transfer commands.
pub mod stock_transfers;
/// Store-profile commands.
pub mod store_profiles;
/// Sync commands.
pub mod sync;
/// Table management commands.
pub mod tables;
/// Tax-rate / tax-rule commands.
pub mod tax;
/// Payment-terminal commands.
pub mod terminals;
/// Void / cancel commands.
pub mod void;
/// Workspace / register layout commands.
pub mod workspaces;
