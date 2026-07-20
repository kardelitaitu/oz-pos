//! Receipt data types and ESC/POS formatting.
//!
//! Defines structured receipt models (`SalesReceipt`) and the
//! `format_sales_receipt` function that produces a ready-to-print
//! byte buffer. Display options are controlled through `ReceiptConfig`.
//!
//! # Layout (80 mm / 48 characters)
//!
//! ```
//! ┌───────────────────────────────────────────────┐
//! │               STORE NAME                │
//! │             123 Main Street              │
//! ├───────────────────────────────────────────────┤
//! │ 01 Jan 2026           #REC-001         │
//! ├───────────────────────────────────────────────┤
//! │ Item                   Qty  Price  Total│
//! │ Milk 2%                  1  $3.50   $3.50│
//! │ Bread White              2  $2.00   $4.00│
//! ├───────────────────────────────────────────────┤
//! │ SUBTOTAL:                      $12.00│
//! │ TAX:                            $1.20│
//! ├───────────────────────────────────────────────┤
//! │ TOTAL:                        $13.20│
//! │                                       │
//! │ CASH:                          $20.00│
//! │ CHANGE:                         $6.80│
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

// ── Decimal separator ────────────────────────────────────

/// How fractional amounts are displayed on receipts.
#[derive(Debug, Clone, Copy)]
pub enum DecimalSeparator {
    /// Period separator: `12.50`
    Dot,
    /// Comma separator: `12,50`
    Comma,
    /// No fractional digits: `12`
    None,
}

impl DecimalSeparator {
    /// Which exponent to use when formatting. `None` means truncate
    /// fractional digits entirely.
    #[must_use]
    pub fn effective_exponent(self, raw: u32) -> Option<usize> {
        match self {
            Self::Dot | Self::Comma => Some(raw as usize),
            Self::None => None,
        }
    }
}

// ── Receipt display configuration ───────────────────────

/// Per-store display options for receipts. Stored in the
/// `settings` table and loaded before each print.
#[derive(Debug, Clone)]
pub struct ReceiptConfig {
    /// Paper width — controls line length.
    pub paper_width: PaperWidth,
    /// Whether to prefix amounts with the currency symbol (e.g. `"$"`).
    pub show_currency: bool,
    /// Decimal separator style.
    pub decimal_separator: DecimalSeparator,
    /// Whether to print a tax line.
    pub show_tax: bool,
    /// Optional footer text (centered at the bottom).
    pub footer: Option<String>,
    /// Whether to print the table number line.
    pub show_table_number: bool,
    /// Whether to print a barcode (receipt number) at the bottom.
    pub barcode_enabled: bool,
    /// Optional payment link template. If set, a QR code is printed
    /// below the barcode. Use `{receipt}` and `{amount}` as placeholders.
    /// Example: `"https://pay.example.com/{receipt}"`
    pub payment_link_template: Option<String>,
}

impl Default for ReceiptConfig {
    fn default() -> Self {
        Self {
            paper_width: PaperWidth::Standard,
            show_currency: false,
            decimal_separator: DecimalSeparator::Dot,
            show_tax: true,
            footer: None,
            show_table_number: false,
            barcode_enabled: false,
            payment_link_template: None,
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
    /// Tax amount for this line (None if tax is not itemised).
    pub tax_amount: Option<Money>,
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
///
/// Display formatting (currency prefix, decimal separator, etc.)
/// is handled by [`ReceiptConfig`] passed to [`format_sales_receipt`].
#[derive(Debug, Clone)]
pub struct SalesReceipt {
    /// Store identity printed at the top.
    pub store: StoreInfo,
    /// Transaction date string (already localised).
    pub date: String,
    /// Sequential receipt / invoice number.
    pub receipt_number: String,
    /// Optional table number (printed after date line).
    pub table_number: Option<String>,
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
}

// ── Helpers ──────────────────────────────────────────────

/// Format a `Money` value according to display config.
fn format_money(m: &Money, config: &ReceiptConfig) -> String {
    let raw_exp = m.currency.minor_unit_exponent() as usize;
    let divisor = 10_i64.pow(raw_exp as u32);
    let sign = if m.minor_units < 0 { "-" } else { "" };
    let abs_val = m.minor_units.unsigned_abs();
    let major = abs_val / divisor as u64;
    let minor = abs_val % divisor as u64;

    let prefix = if config.show_currency {
        currency_symbol(&m.currency)
    } else {
        ""
    };

    match config.decimal_separator {
        DecimalSeparator::None => {
            format!("{sign}{prefix}{major}")
        }
        DecimalSeparator::Comma => {
            format!("{sign}{prefix}{major},{minor:0width$}", width = raw_exp)
        }
        DecimalSeparator::Dot => {
            format!("{sign}{prefix}{major}.{minor:0width$}", width = raw_exp)
        }
    }
}

/// Best-effort currency symbol for the given ISO-4217 code.
/// Falls back to the code itself if no common symbol is known.
fn currency_symbol(currency: &oz_core::Currency) -> &'static str {
    let code = std::str::from_utf8(&currency.0).unwrap_or("  ");
    match code {
        "USD" | "SGD" | "HKD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        "IDR" => "Rp",
        "MYR" => "RM",
        "PHP" => "₱",
        "THB" => "฿",
        "KRW" => "₩",
        "BRL" => "R$",
        _ => "$",
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

    fn barcode(&mut self, barcode_type: escpos::BarcodeType, data: &[u8]) {
        self.left();
        self.blank();
        self.buf
            .extend_from_slice(&escpos::barcode(barcode_type, data));
        self.buf.extend_from_slice(escpos::LF);
    }

    fn qr_code(&mut self, data: &[u8], module_size: u8) {
        self.blank();
        self.buf
            .extend_from_slice(&escpos::qr_code(data, module_size));
        self.buf.extend_from_slice(escpos::LF);
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
            32 => Self {
                name: 16,
                qty: 3,
                price: 5,
                total: 5,
                sep: " ",
            },
            _ => Self {
                name: 26,
                qty: 4,
                price: 6,
                total: 6,
                sep: "  ",
            },
        }
    }
}

// ── Public formatter ─────────────────────────────────────

/// Build an ESC/POS byte buffer for a sales receipt.
///
/// `config` controls display options (currency prefix, decimal
/// separator, paper width, tax visibility, footer text).
///
/// The returned buffer can be sent directly to any printer via
/// `ReceiptPrinter::print_raw` — it includes the initialisation
/// sequence, all text and formatting commands, a 3-line paper feed,
/// and a full paper cut.
pub fn format_sales_receipt(r: &SalesReceipt, config: &ReceiptConfig) -> Vec<u8> {
    let w = config.paper_width.chars();
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
    let right_text = format!("#{}", r.receipt_number);
    let left_text = &r.date;
    let gap = " ".repeat(w.saturating_sub(left_text.len() + right_text.len() + 1));
    b.text(&format!("{left_text}{gap}{right_text}"));

    // ── Table number (optional) ────────────────────────
    if config.show_table_number
        && let Some(ref tn) = r.table_number
    {
        let table_line = format!("Table: {tn}");
        let table_gap = " ".repeat(w.saturating_sub(table_line.len()));
        b.text(&format!("{table_gap}{table_line}"));
    }

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
            "",
            s = cols.sep,
        );
        b.bold(&header);
    }

    // ── Line items ────────────────────────────────────
    for item in &r.items {
        let name = truncate(&item.name, cols.name);
        let qty_s = format!("{}", item.quantity);
        let price_s = format_money(&item.unit_price, config);
        let total_s = format_money(&item.total_price, config);

        let qty_pad = cols.qty.saturating_sub(qty_s.len());
        let price_pad = cols.price.saturating_sub(price_s.len());
        let total_pad = cols.total.saturating_sub(total_s.len());

        let line = format!(
            "{:<name$}{sep}{:>qty_pad$}{qty_s}{sep}{:>price_pad$}{price_s}{sep}{:>total_pad$}{total_s}",
            name,
            "",
            "",
            "",
            name = cols.name,
            sep = cols.sep,
            qty_pad = qty_pad,
            price_pad = price_pad,
            total_pad = total_pad,
        );
        b.text(&line);
        if config.show_tax
            && let Some(ref tax) = item.tax_amount
        {
            let indent = cols.name
                + cols.sep.len()
                + cols.qty
                + cols.sep.len()
                + cols.price
                + cols.sep.len();
            let tax_str = format_money(tax, config);
            let tax_line = format!(
                "{:indent$}Tax: {:>tax_pad$}{tax_str}",
                "",
                "",
                indent = indent,
                tax_pad = cols.total.saturating_sub(tax_str.len() + 5)
            );
            b.text(&tax_line);
        }
    }
    b.separator();

    // ── Totals (right-aligned) ────────────────────────
    b.text(&right_line(
        "SUBTOTAL:",
        &format_money(&r.subtotal, config),
        w,
    ));
    if config.show_tax
        && let Some(ref tax) = r.tax
    {
        b.text(&right_line("TAX:", &format_money(tax, config), w));
    }
    b.separator();
    b.bold(&right_line("TOTAL:", &format_money(&r.total, config), w));
    b.blank();

    // ── Payments ──────────────────────────────────────
    for pmt in &r.payments {
        b.text(&right_line(
            &pmt.method.to_uppercase(),
            &format_money(&pmt.amount, config),
            w,
        ));
        if let Some(ref chg) = pmt.change {
            b.text(&right_line("CHANGE:", &format_money(chg, config), w));
        }
    }

    // ── Footer ────────────────────────────────────────
    if let Some(ref footer) = config.footer {
        b.separator();
        b.center(footer);
    }

    b.blank();

    // ── Barcode (receipt number) ──────────────────
    if config.barcode_enabled {
        let receipt_barcode = format!("#{}", r.receipt_number);
        b.barcode(escpos::BarcodeType::Code128, receipt_barcode.as_bytes());
    }

    // ── QR code (payment link) ────────────────────
    if let Some(ref template) = config.payment_link_template {
        let qr_data = template
            .replace("{receipt}", &r.receipt_number)
            .replace("{amount}", &r.total.minor_units.to_string());
        if !qr_data.is_empty() {
            b.qr_code(qr_data.as_bytes(), 5);
        }
    }

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

    fn default_config() -> ReceiptConfig {
        ReceiptConfig::default()
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
            table_number: None,
            items: vec![
                LineItem {
                    name: "Milk 2%".into(),
                    quantity: 1,
                    unit_price: usd_money(350),
                    total_price: usd_money(350),
                    tax_amount: Some(usd_money(35)),
                },
                LineItem {
                    name: "Bread White".into(),
                    quantity: 2,
                    unit_price: usd_money(200),
                    total_price: usd_money(400),
                    tax_amount: Some(usd_money(40)),
                },
                LineItem {
                    name: "Eggs (dozen)".into(),
                    quantity: 1,
                    unit_price: usd_money(450),
                    total_price: usd_money(450),
                    tax_amount: Some(usd_money(45)),
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
        }
    }

    #[test]
    fn format_money_dot_default() {
        let cfg = default_config();
        assert_eq!(format_money(&usd_money(1550), &cfg), "15.50");
        assert_eq!(format_money(&usd_money(0), &cfg), "0.00");
        assert_eq!(format_money(&usd_money(100), &cfg), "1.00");
    }

    #[test]
    fn format_money_comma() {
        let cfg = ReceiptConfig {
            decimal_separator: DecimalSeparator::Comma,
            ..default_config()
        };
        assert_eq!(format_money(&usd_money(1550), &cfg), "15,50");
    }

    #[test]
    fn format_money_no_decimals() {
        let cfg = ReceiptConfig {
            decimal_separator: DecimalSeparator::None,
            ..default_config()
        };
        assert_eq!(format_money(&usd_money(1550), &cfg), "15");
        assert_eq!(format_money(&usd_money(100), &cfg), "1");
    }

    #[test]
    fn format_money_with_currency() {
        let cfg = ReceiptConfig {
            show_currency: true,
            ..default_config()
        };
        assert_eq!(format_money(&usd_money(1550), &cfg), "$15.50");
    }

    #[test]
    fn format_money_with_currency_no_decimals() {
        let cfg = ReceiptConfig {
            show_currency: true,
            decimal_separator: DecimalSeparator::None,
            ..default_config()
        };
        assert_eq!(format_money(&usd_money(2000), &cfg), "$20");
    }

    #[test]
    fn format_money_negative() {
        let cfg = default_config();
        let m = Money {
            minor_units: -1550,
            currency: "USD".parse::<Currency>().unwrap(),
        };
        assert_eq!(format_money(&m, &cfg), "-15.50");
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
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("OZ MART"));
    }

    #[test]
    fn sales_receipt_contains_receipt_number() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("#REC-001"));
    }

    #[test]
    fn sales_receipt_contains_item_names() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("Milk 2%"));
        assert!(text.contains("Bread White"));
        assert!(text.contains("Eggs (dozen)"));
    }

    #[test]
    fn sales_receipt_contains_total() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("13.20"));
    }

    #[test]
    fn sales_receipt_contains_tax_when_show_tax() {
        let cfg = ReceiptConfig {
            show_tax: true,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("TAX:"));
        assert!(text.contains("1.20"));
    }

    #[test]
    fn sales_receipt_hides_tax_when_show_tax_false() {
        let cfg = ReceiptConfig {
            show_tax: false,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            !text.contains("TAX:"),
            "tax should not appear when show_tax=false"
        );
    }

    #[test]
    fn sales_receipt_shows_per_line_tax() {
        let cfg = ReceiptConfig {
            show_tax: true,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("Tax:"), "per-line tax label should appear");
        assert!(text.contains("0.35"), "milk tax should appear");
        assert!(text.contains("0.40"), "bread tax should appear");
        assert!(text.contains("0.45"), "egg tax should appear");
    }

    #[test]
    fn sales_receipt_hides_per_line_tax_when_show_tax_false() {
        let cfg = ReceiptConfig {
            show_tax: false,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            !text.contains("Tax:"),
            "per-line tax should not appear when show_tax=false"
        );
    }

    #[test]
    fn sales_receipt_contains_currency_when_enabled() {
        let cfg = ReceiptConfig {
            show_currency: true,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            text.contains("$13.20"),
            "receipt should show $ prefix: {:?}",
            text
        );
    }

    #[test]
    fn sales_receipt_contains_payment_and_change() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("CASH"));
        assert!(text.contains("20.00"));
        assert!(text.contains("CHANGE:"));
        assert!(text.contains("6.80"));
    }

    #[test]
    fn sales_receipt_contains_footer() {
        let cfg = ReceiptConfig {
            footer: Some("Thank you for shopping!".into()),
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(text.contains("Thank you for shopping!"));
    }

    #[test]
    fn sales_receipt_starts_with_esc_init() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        assert!(data.starts_with(escpos::ESC_INIT));
    }

    #[test]
    fn sales_receipt_ends_with_cut() {
        let data = format_sales_receipt(&sample_receipt(), &default_config());
        assert!(data.ends_with(escpos::CUT_FULL));
    }

    #[test]
    fn narrow_width_uses_32_chars() {
        let cfg = ReceiptConfig {
            paper_width: PaperWidth::Narrow,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
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

    #[test]
    fn decimal_separator_effective_exponent() {
        assert_eq!(DecimalSeparator::Dot.effective_exponent(2), Some(2));
        assert_eq!(DecimalSeparator::Comma.effective_exponent(3), Some(3));
        assert_eq!(DecimalSeparator::None.effective_exponent(2), None);
    }

    #[test]
    fn currency_symbol_known_codes() {
        let usd: oz_core::Currency = "USD".parse().unwrap();
        let eur: oz_core::Currency = "EUR".parse().unwrap();
        let idr: oz_core::Currency = "IDR".parse().unwrap();
        assert_eq!(currency_symbol(&usd), "$");
        assert_eq!(currency_symbol(&eur), "€");
        assert_eq!(currency_symbol(&idr), "Rp");
    }

    #[test]
    fn receipt_prints_table_number_when_enabled_and_provided() {
        let cfg = ReceiptConfig {
            show_table_number: true,
            ..default_config()
        };
        let mut r = sample_receipt();
        r.table_number = Some("5".into());
        let data = format_sales_receipt(&r, &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            text.contains("Table: 5"),
            "receipt should contain 'Table: 5'"
        );
    }

    #[test]
    fn receipt_hides_table_number_when_disabled() {
        let cfg = ReceiptConfig {
            show_table_number: false,
            ..default_config()
        };
        let mut r = sample_receipt();
        r.table_number = Some("5".into());
        let data = format_sales_receipt(&r, &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            !text.contains("Table:"),
            "receipt should not contain 'Table:'"
        );
    }

    #[test]
    fn receipt_hides_table_number_when_none() {
        let cfg = ReceiptConfig {
            show_table_number: true,
            ..default_config()
        };
        let mut r = sample_receipt();
        r.table_number = None;
        let data = format_sales_receipt(&r, &cfg);
        let text = String::from_utf8_lossy(&data);
        assert!(
            !text.contains("Table:"),
            "receipt should not contain 'Table:'"
        );
    }

    // ── Barcode & QR code tests ───────────────────────────────────────

    #[test]
    fn barcode_appears_when_enabled() {
        let cfg = ReceiptConfig {
            barcode_enabled: true,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        // Should contain GS h A0 (barcode height command)
        assert!(
            data.windows(3).any(|w| w == [0x1D, 0x68, 0xA0]),
            "missing GS h barcode height command"
        );
        // Should contain GS k (barcode print command)
        assert!(
            data.windows(2).any(|w| w == [0x1D, 0x6B]),
            "missing GS k barcode print command"
        );
        // Should contain the receipt number data
        let receipt_bytes = b"#REC-001";
        assert!(
            data.windows(receipt_bytes.len())
                .any(|w| w == receipt_bytes),
            "barcode should encode receipt number"
        );
    }

    #[test]
    fn barcode_omitted_when_disabled() {
        let cfg = ReceiptConfig {
            barcode_enabled: false,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        // Should NOT contain GS h A0 (barcode command prefix)
        // But could contain other GS commands, so we check for GS h specifically
        let gs_h_count = data.windows(2).filter(|w| *w == [0x1D, 0x68]).count();
        assert_eq!(gs_h_count, 0, "no GS h commands expected");
    }

    #[test]
    fn qr_code_appears_when_template_provided() {
        let cfg = ReceiptConfig {
            payment_link_template: Some("https://pay.example.com/{receipt}".into()),
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        // Should contain GS ( k (QR code command prefix)
        assert!(
            data.windows(3).any(|w| w == [0x1D, 0x28, 0x6B]),
            "missing GS ( k QR code commands"
        );
        // Should contain the payment URL
        let url = b"https://pay.example.com/REC-001";
        assert!(
            data.windows(url.len()).any(|w| w == url),
            "QR code should contain payment URL with receipt number"
        );
    }

    #[test]
    fn qr_code_with_amount_placeholder() {
        let cfg = ReceiptConfig {
            payment_link_template: Some("https://pay.example.com/{receipt}/{amount}".into()),
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let expected = b"https://pay.example.com/REC-001/1320";
        assert!(
            data.windows(expected.len()).any(|w| w == expected),
            "QR should encode URL with amount"
        );
    }

    #[test]
    fn qr_code_omitted_when_template_none() {
        let cfg = ReceiptConfig {
            payment_link_template: None,
            ..default_config()
        };
        let data = format_sales_receipt(&sample_receipt(), &cfg);
        let url = b"pay.example.com";
        assert!(
            !data.windows(url.len()).any(|w| w == url),
            "QR payment URL should not appear when template is None"
        );
    }

    #[test]
    fn barcode_and_qr_both_appear_when_configured() {
        let item = sample_receipt();
        let cfg = ReceiptConfig {
            barcode_enabled: true,
            payment_link_template: Some("https://pay.example.com/qr".into()),
            ..default_config()
        };
        let data = format_sales_receipt(&item, &cfg);
        // Both barcode and QR commands should appear
        assert!(
            data.windows(2).any(|w| w == [0x1D, 0x6B]),
            "barcode command missing"
        );
        assert!(
            data.windows(3).any(|w| w == [0x1D, 0x28, 0x6B]),
            "QR command missing"
        );
    }
}
