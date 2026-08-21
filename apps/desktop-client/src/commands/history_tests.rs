use super::*;
use foundation::{Currency, Money};
use oz_core::SaleLine;

fn usd() -> Currency {
    "USD".parse().unwrap()
}

fn price(minor: i64) -> Money {
    Money {
        minor_units: minor,
        currency: usd(),
    }
}

fn make_sale_line(sale_id: &str, sku: &str, qty: i64, unit: i64) -> SaleLine {
    SaleLine {
        id: uuid::Uuid::now_v7().to_string(),
        sale_id: sale_id.into(),
        sku: sku.into(),
        qty,
        unit_price: price(unit),
        line_total: price(unit * qty),
        line_position: 1,
        tax_amount: Money::zero(usd()),
        tax_rate_id: None,
        tax_breakdown_json: None,
        serial_number: None,
        course: None,
        modifiers_json: None,
    }
}

// ── SaleListItem ───────────────────────────────────────────────────

#[test]
fn sale_list_item_debug() {
    let item = SaleListItem {
        id: "s1".into(),
        total: price(5000),
        line_count: 3,
        status: "completed".into(),
        payment_method: Some("cash".into()),
        user_id: Some("u1".into()),
        created_at: "2025-01-01".into(),
    };
    let d = format!("{item:?}");
    assert!(d.contains("s1"));
    assert!(d.contains("5000"));
    assert!(d.contains("completed"));
}

#[test]
fn sale_list_item_serialize() {
    let item = SaleListItem {
        id: "s2".into(),
        total: price(7500),
        line_count: 1,
        status: "voided".into(),
        payment_method: None,
        user_id: None,
        created_at: "2025-06-01".into(),
    };
    let json = serde_json::to_value(&item).unwrap();
    assert_eq!(json["id"], "s2");
    assert_eq!(json["status"], "voided");
    assert!(json["payment_method"].is_null());
}

// ── SaleDetail ──────────────────────────────────────────────────────

#[test]
fn sale_detail_debug() {
    let detail = SaleDetail {
        id: "sd1".into(),
        total: price(10000),
        subtotal: price(10000),
        tax_total: Money::zero(usd()),
        line_count: 2,
        status: "completed".into(),
        payment_method: Some("card".into()),
        tendered_minor: Some(12000),
        user_id: Some("u2".into()),
        created_at: "2025-03-15".into(),
        lines: vec![make_sale_line("sd1", "SKU-A", 2, 5000)],
    };
    let d = format!("{detail:?}");
    assert!(d.contains("sd1"));
    assert!(d.contains("SKU-A"));
}

#[test]
fn sale_detail_serialize() {
    let detail = SaleDetail {
        id: "sd2".into(),
        total: price(3000),
        subtotal: price(3000),
        tax_total: Money::zero(usd()),
        line_count: 1,
        status: "completed".into(),
        payment_method: None,
        tendered_minor: None,
        user_id: None,
        created_at: "2025-01-01".into(),
        lines: vec![],
    };
    let json = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["id"], "sd2");
    assert!(json["tendered_minor"].is_null());
    assert_eq!(json["lines"].as_array().unwrap().len(), 0);
}

// ── PaymentBreakdown ────────────────────────────────────────────────

#[test]
fn payment_breakdown_debug() {
    let pb = PaymentBreakdown {
        method: "cash".into(),
        count: 15,
        total: 50000,
    };
    let d = format!("{pb:?}");
    assert!(d.contains("cash"));
    assert!(d.contains("15"));
}

#[test]
fn payment_breakdown_serialize() {
    let pb = PaymentBreakdown {
        method: "card".into(),
        count: 42,
        total: 120000,
    };
    let json = serde_json::to_value(&pb).unwrap();
    assert_eq!(json["method"], "card");
    assert_eq!(json["count"], 42);
    assert_eq!(json["total"], 120000);
}

// ── EodReport ───────────────────────────────────────────────────────

#[test]
fn eod_report_debug() {
    let report = EodReport {
        total_sales: 100,
        total_revenue: 500000,
        currency: "IDR".into(),
        payment_breakdown: vec![],
        void_count: 3,
        void_total: 15000,
        discount_count: 10,
        discount_total: 25000,
        hourly_breakdown: vec![],
    };
    let d = format!("{report:?}");
    assert!(d.contains("100"));
    assert!(d.contains("IDR"));
}

#[test]
fn eod_report_serialize() {
    let report = EodReport {
        total_sales: 50,
        total_revenue: 250000,
        currency: "USD".into(),
        payment_breakdown: vec![PaymentBreakdown {
            method: "cash".into(),
            count: 30,
            total: 150000,
        }],
        void_count: 1,
        void_total: 5000,
        discount_count: 5,
        discount_total: 10000,
        hourly_breakdown: vec![],
    };
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["total_sales"], 50);
    assert_eq!(json["currency"], "USD");
    assert!(!json["payment_breakdown"].as_array().unwrap().is_empty());
}

// ── Session token rejection tests ─────────────────────────────────

#[test]
fn list_sales_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[test]
fn get_sale_scoped_rejects_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("bad-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}

#[test]
fn export_reports_scoped_reject_invalid_token() {
    let state = AppState::for_test();
    let result = state.resolve_session("nonexistent-token");
    assert!(matches!(result, Err(AppError::InvalidSession)));
}
