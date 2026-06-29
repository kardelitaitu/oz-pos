//! Backward-compat re-exports from the split command modules.
//!
//! The old monolithic `commands/sales.rs` has been split into:
//! - `commands/pos.rs`     — start_sale, add_line, complete_sale, set_cart_discount, held carts
//! - `commands/history.rs`  — list_sales, get_sale, export reports, EOD
//! - `commands/void.rs`     — void_sale
//!
//! This file re-exports everything for callers that haven't migrated yet.

pub use super::pos::{
    AddLineArgs, AddLineResult, CompleteSaleArgs, CompleteSaleResult, HoldCartArgs, HoldCartResult,
    SetCartDiscountArgs, StartSaleArgs, StartSaleResult, add_line, complete_sale, delete_held_cart,
    get_held_cart, hold_cart, list_held_carts, set_cart_discount, start_sale,
};

pub use super::history::{
    EodReport, PaymentBreakdown, SaleDetail, SaleListItem, export_daily_summary, export_eod_report,
    export_sales_by_hour, get_sale, list_sales,
};

pub use super::void::{VoidSaleArgs, void_sale};
