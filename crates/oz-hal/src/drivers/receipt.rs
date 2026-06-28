//! Receipt data types and ESC/POS formatting.
//!
//! Defines structured receipt models ([`SalesReceipt`]) and the
//! [`format_sales_receipt`] function that produces a ready-to-print
//! byte buffer.
//!
//! # Layout (80 mm / 48 characters)
//!
//! ```text
//! ┌───────────────────────────────────────────────┐
//! │               STORE NAME                │
//! │             123 Main Street              │
//! ├───────────────────────────────────────────────┤
//! │ 01 Jan 2026           #REC-001         │
//! ├───────────────────────────────────────────────┤
//! │ Item                   Qty  Price  Total│
//! │ Milk 2%                  1   3.50    3.50│
//! │ Bread White              2   2.00    4.00│
//! ├───────────────────────────────────────────────┤
//! │ SUBTOTAL:                        15.00│
//! │ TAX (10%):                       1.50│
//! ├───────────────────────────────────────────────┤
//! │ TOTAL:                          16.50│
//! │                                       │
//! │ CASH:                            20.00│
//! │ CHANGE:                           3.50│
//! ├───────────────────────────────────────────────┤
//! │        Thanks for shopping!             │
//! └───────────────────────────────────────────────┘
//! ```

use oz_core::Money;

use super::escpos;

// ── Paper width ──────────────────────────────────────────

/// Thermal paper width presets.
#[derive(Debug, Clone, Copy)]
pub enum PaperWidth {
    /// ~58 mm paper (~32 monospace characters).
    Narrow,
    /// ~80 mm paper (~48 monospace characters).
    Standard,
}

impl PaperWidth {
    /// Maximum number of ASCII characters per line.
    #[must_use]
    pub fn chars(self) -> usize {
        match self {
            Self::Narrow => 32,
            Self::Standard => 48,
        }
    }
}

// ── Store info ───────────────────────────────────────────

/// Store information printed at the top of every receipt.
#[derive(Debug, Clone)]
pub struct StoreInfo {
    /// Store display name.
    pub name: String,
    /// Street address line(s), joined with ` / `.
    pub address: String,
    /// Optional tax registration number.
    pub tax_id: Option<String>,
}

// ── Line item ────────────────────────────────────────────

/// A single product line on a receipt.
#[derive(Debug, Clone)]
pub struct LineItem {
    /// Product display name.
    pub name: String,
    /// Quantity purchased.
    pub quantity: u32,
    /// Price per unit.
    pub unit_price: Money,
    /// Quantity × unit price.
    pub total_price: Money,
}

// ── Payment info ─────────────────────────────────────────

/// A payment applied to the receipt.
#[derive(Debug, Clone)]
pub struct PaymentInfo {
    /// Payment method label (e.g. `"CASH"`, `"CARD"`, `"QRIS"`).
    pub method: String,
    /// Amount tendered.
    pub amount: Money,
    /// Change returned, if applicable.
    pub change: Option<Money>,
}

// ── Sales receipt ────────────────────────────────────────

/// A complete sales receipt ready to format and print.
#[derive(Debug, Clone)]
pub struct SalesReceipt {
    /// Store identity printed at the top.
    pub store: StoreInfo,
    /// Transaction date string (already localised).
    pub date: String,
    /// Sequential receipt / invoice number.
    pub receipt_number: String,
    /// Purchased line items.
    pub items: Vec<LineItem>,
    /// Subtotal before tax.
    pub subtotal: Money,
    /// Tax amount (None => no tax line).
    pub tax: Option<Money>,
    /// Grand total (subtotal + tax).
    pub total: Money,
    /// Payments tendered.
    pub payments: Vec<PaymentInfo>,
    /// Optional footer message (e.g. "Thank you, please come again").
    pub footer: Option<String>,
    /// Paper width preset.
    pub paper_width: PaperWidth,
}

// ── Helpers ──────────────────────────────────────────────

/// Format a `Money` value as a string with the correct number of
/// decimal places for its currency (e.g. `"15.50"` for USD).
fn format_money(m: &Money) -> String {
    let exp = m.currency.minor_unit_exponent() as usize;
    let divisor = 10_i64.pow(exp as u32);
    let major = m.minor_units / divisor;
    let minor = (m.minor_units % divisor).unsigned_abs();
    if m.minor_units < 0 {
        format!("-{}.{minor:0width$}", major.unsigned_abs(), width = exp)
    } else {
        format!("{major}.{minor:0width$}", width = exp)
    }
}

/// Truncate a string to at most `max` characters, appending `…` if
/// truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else if max > 1 {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        "…".to_owned()
    }
}

// ── Builder ──────────────────────────────────────────────

/// Internal builder that accumulates ESC/POS bytes.
struct ReceiptBuilder {
    buf: Vec<u8>,
    width: usize,
}

impl ReceiptBuilder {
    fn new(width: usize) -> Self {
        Self {
            buf: Vec::with_capacity(2048),
            width,
        }
    }

    fn init(&mut self) {
        self.buf.extend_from_slice(escpos::ESC_INIT);
        self.buf.extend_from_slice(escpos::LINE_SPACING_DEFAULT);
        self.buf.extend_from_slice(escpos::FONT_A);
    }

    fn left(&mut self) {
        self.buf.extend_from_slice(escpos::ALIGN_LEFT);
    }

    fn text(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.extend_from_slice(escpos::LF);
    }

    fn center(&mut self, s: &str) {
        self.buf.extend_from_slice(escpos::ALIGN_CENTER);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.extend_from_slice(escpos::LF);
        self.left();
    }

    fn bold_center(&mut self, s: &str) {
        self.buf.extend_from_slice(escpos::BOLD_ON);
        self.center(s);
        self.buf.extend_from_slice(escpos::BOLD_OFF);
    }

    fn bold(&mut self, s: &str) {
        self.buf.extend_from_slice(escpos::BOLD_ON);
        self.text(s);
        self.buf.extend_from_slice(escpos::BOLD_OFF);
    }

    fn separator(&mut self) {
        self.text(&"─".repeat(self.width));
    }

    fn blank(&mut self) {
        self.text("");
    }

    fn feed(&mut self, n: u8) {
        self.buf.extend_from_slice(&escpos::feed(n));
    }

    fn cut(&mut self) {
        self.buf.extend_from_slice(escpos::CUT_FULL);
    }

    fn build(self) -> Vec<u8> {
        self.buf
    }
}

// ── Column layout helpers ────────────────────────────────

/// Column widths for the item table, keyed by paper width.
struct TableCols {
    name: usize,
    qty: usize,
    price: usize,
    total: usize,
    sep: &'static str,
}

impl TableCols {
    fn for_width(w: usize) -> Self {
        match w {
            32 => Self { name: 16, qty: 3, price: 5, total: 5, sep: " " },
            _ => Self { name: 26, qty: 4, price: 6, total: 6, sep: "  " },
        }
    }

}

// ── Public formatter ─────────────────────────────────────

/// Build an ESC/POS byte buffer for a sales receipt.
///
/// The returned buffer can be sent directly to any printer via
/// [`ReceiptPrinter::print_raw`] — it includes the initialisation
/// sequence, all text and formatting commands, a 3-line paper feed,
/// and a full paper cut.
pub fn format_sales_receipt(r: &SalesReceipt) -> Vec<u8> {
    let w = r.paper_width.chars();
    let mut b = ReceiptBuilder::new(w);

    b.init();
    b.blank();

    // ── Header (centered + bold) ──────────────────────
    b.bold_center(&r.store.name);
    for line in r.store.address.split('/') {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            b.center(trimmed);
        }
    }
    if let Some(ref tax_id) = r.store.tax_id {
        b.center(&format!("NPWP: {tax_id}"));
    }
    b.blank();
    b.separator();

    // ── Date / receipt number ─────────────────────────
    // Left-aligned date, right-aligned receipt number.
    let right_text = format!("#{}", r.receipt_number);
    let left_text = &r.date;
    let gap = " ".repeat(w.saturating_sub(left_text.len() + right_text.len() + 1));
    b.text(&format!("{left_text}{gap}{right_text}"));
    b.separator();

    // ── Column headers ────────────────────────────────
    let cols = TableCols::for_width(w);
    {
        let name_w = cols.name.saturating_sub(4);
        let qty_h = right_pad("Qty", cols.qty);
        let price_h = right_pad("Price", cols.price);
        let total_h = right_pad("Total", cols.total);
        let header = format!(
            "Item{:<name_w$}{s}{qty_h}{s}{price_h}{s}{total_h}",
            "", s = cols.sep,
        );
        b.bold(&header);
    }

    // ── Line items ────────────────────────────────────
    for item in &r.items {
        let name = truncate(&item.name, cols.name);
        let qty_s = format!("{}", item.quantity);
        let price_s = format_money(&item.unit_price);
        let total_s = format_money(&item.total_price);

        // Manual column alignment with right-aligned numerics
        let qty_pad = cols.qty.saturating_sub(qty_s.len());
        let price_pad = cols.price.saturating_sub(price_s.len());
        let total_pad = cols.total.saturating_sub(total_s.len());

        let line = format!(
            "{:<name$}{sep}{:>qty_pad$}{qty_s}{sep}{:>price_pad$}{price_s}{sep}{:>total_pad$}{total_s}",
            name, "", "", "",
            name = cols.name,
            sep = cols.sep,
            qty_pad = qty_pad,
            price_pad = price_pad,
            total_pad = total_pad,
        );
        b.text(&line);
    }
    b.separator();

    // ── Totals (right-aligned) ────────────────────────
    b.text(&right_line("SUBTOTAL:", &format_money(&r.subtotal), w));
    if let Some(ref tax) = r.tax {
        b.text(&right_line("TAX:", &format_money(tax), w));
    }
    b.separator();
    b.bold(&right_line("TOTAL:", &format_money(&r.total), w));
    b.blank();

    // ── Payments ──────────────────────────────────────
    for pmt in &r.payments {
        b.text(&right_line(&pmt.method.to_uppercase(), &format_money(&pmt.amount), w));
        if let Some(ref chg) = pmt.change {
            b.text(&right_line("CHANGE:", &format_money(chg), w));
        }
    }

    // ── Footer ────────────────────────────────────────
    if let Some(ref footer) = r.footer {
        b.separator();
        b.center(footer);
    }

    b.blank();
    b.feed(3);
    b.cut();
    b.build()
}

/// Right-pad a string to at least `width` characters with leading spaces.
fn right_pad(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_owned()
    } else {
        format!("{:>width$}", s, width = width)
    }
}

/// Line with right-aligned value: `"LABEL         12.50"`
fn right_line(label: &str, value: &str, width: usize) -> String {
    let content_w = label.len() + 1 + value.len();
    if content_w >= width {
        format!("{label} {value}")
    } else {
        let gap = " ".repeat(width - content_w);
        format!("{label}{gap}{value}")
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use oz_core::Currency;

    use super::*;

    fn usd_money(amount: i64) -> Money {
        Money {
            minor_units: amount,
            currency: "USD".parse::<Currency>().unwrap(),
        }
    }

    fn sample_receipt() -> SalesReceipt {
        SalesReceipt {
            store: StoreInfo {
                name: "OZ MART".into(),
                address: "123 Main Street / Springfield, IL 62701".into(),
                tax_id: Some("12-3456789".into()),
            },
            date: "01 Jan 2026".into(),
            receipt_number: "REC-001".into(),
            items: vec![
                LineItem {
                    name: "Milk 2%".into(),
                    quantity: 1,
                    unit_price: usd_money(350),
                    total_price: usd_money(350),
                },
                LineItem {
                    name: "Bread White".into(),
                    quantity: 2,
                    unit_price: usd_money(200),
                    total_price: usd_money(400),
                },
                LineItem {
                    name: "Eggs (dozen)".into(),
                    quantity: 1,
                    unit_price: usd_money(450),
                    total_price: usd_money(450),
                },
            ],
            subtotal: usd_money(1200),
            tax: Some(usd_money(120)),
            total: usd_money(1320),
            payments: vec![PaymentInfo {
                method: "CASH".into(),
                amount: usd_money(2000),
                change: Some(usd_money(680)),
            }],
            footer: Some("Thank you for shopping!".into()),
            paper_width: PaperWidth::Standard,
        }
    }

    #[test]
    fn format_money_usd() {
        assert_eq!(format_money(&usd_money(1550)), "15.50");
        assert_eq!(format_money(&usd_money(0)), "0.00");
        assert_eq!(format_money(&usd_money(100)), "1.00");
    }

    #[test]
    fn format_money_negative() {
        let m = Money {
            minor_units: -1550,
            currency: "USD".parse::<Currency>().unwrap(),
        };
        assert_eq!(format_money(&m), "-15.50");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("Hello", 10), "Hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("Hello World", 8), "Hello W…");
    }

    #[test]
    fn sales_receipt_contains_store_name() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("OZ MART"));
    }

    #[test]
    fn sales_receipt_contains_receipt_number() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("#REC-001"));
    }

    #[test]
    fn sales_receipt_contains_item_names() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("Milk 2%"));
        assert!(text.contains("Bread White"));
        assert!(text.contains("Eggs (dozen)"));
    }

    #[test]
    fn sales_receipt_contains_total() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("13.20"));
    }

    #[test]
    fn sales_receipt_contains_tax() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("TAX:"));
        assert!(text.contains("1.20"));
    }

    #[test]
    fn sales_receipt_contains_payment_and_change() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("CASH"));
        assert!(text.contains("20.00"));
        assert!(text.contains("CHANGE:"));
        assert!(text.contains("6.80"));
    }

    #[test]
    fn sales_receipt_contains_footer() {
        let data = format_sales_receipt(&sample_receipt());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("Thank you for shopping!"));
    }

    #[test]
    fn sales_receipt_starts_with_esc_init() {
        let data = format_sales_receipt(&sample_receipt());
        assert!(data.starts_with(escpos::ESC_INIT));
    }

    #[test]
    fn sales_receipt_ends_with_cut() {
        let data = format_sales_receipt(&sample_receipt());
        assert!(data.ends_with(escpos::CUT_FULL));
    }

    #[test]
    fn narrow_width_uses_32_chars() {
        let mut r = sample_receipt();
        r.paper_width = PaperWidth::Narrow;
        let data = format_sales_receipt(&r);
        // Separator should be 32 dashes
        let text = String::from_utf8_lossy(&data);
        for line in text.lines() {
            let dash_count = line.chars().filter(|&c| c == '─').count();
            if dash_count > 0 {
                assert!(dash_count <= 32, "separator too long: {dash_count}");
            }
        }
    }

    #[test]
    fn right_line_pads_correctly() {
        let result = right_line("TOTAL:", "13.20", 48);
        assert!(result.starts_with("TOTAL:"));
        assert!(result.ends_with("13.20"));
    }
}
