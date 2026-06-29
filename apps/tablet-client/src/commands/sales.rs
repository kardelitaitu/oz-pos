//! Backward-compat re-exports from the split command modules.
//!
//! The old monolithic `commands/sales.rs` has been split into:
//! - `commands/pos.rs`     — start_sale, add_line, complete_sale, set_cart_discount, held carts
//! - `commands/history.rs`  — list_sales, get_sale, export reports, EOD
//! - `commands/void.rs`     — void_sale
//!
//! This file re-exports everything for callers that haven't migrated yet.

pub use super::pos::{
    start_sale, add_line, complete_sale, set_cart_discount,
    hold_cart, list_held_carts, get_held_cart, delete_held_cart,
    StartSaleArgs, StartSaleResult, AddLineArgs, AddLineResult,
    CompleteSaleArgs, CompleteSaleResult, SetCartDiscountArgs,
    HoldCartArgs, HoldCartResult,
};

pub use super::history::{
    list_sales, get_sale, export_daily_summary, export_sales_by_hour, export_eod_report,
    SaleListItem, SaleDetail, EodReport, PaymentBreakdown,
};

pub use super::void::{
    void_sale, VoidSaleArgs,
};
