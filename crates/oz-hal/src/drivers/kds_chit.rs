//! KDS kitchen chit formatting — produces ESC/POS bytes for thermal
//! printers in the kitchen.
//!
//! Chits are compact, focused on what the kitchen needs: order number,
//! table number, items, notes, and timestamps. Designed for 80mm
//! thermal paper (~48 characters wide).

use super::escpos;

/// A formatted kitchen chit ready to send to a thermal printer.
pub struct KdsChit {
    /// ESC/POS bytes (includes init, text, feed, and cut commands).
    pub data: Vec<u8>,
    /// Plain-text representation (for logging/display).
    pub text: String,
}

/// Format a KDS order into a printable kitchen chit.
///
/// The chit includes:
/// - Header with "KITCHEN ORDER" label
/// - Order number and table number
/// - Item summary (what the kitchen needs to cook)
/// - Notes (allergies, modifications)
/// - Timestamp of when the order was received
/// - Separator lines for visual clarity
pub fn format_kds_chit(
    display_number: Option<i64>,
    table_number: Option<&str>,
    items_summary: &str,
    item_count: i64,
    notes: &str,
    received_at: &str,
) -> KdsChit {
    let w = 48; // 80mm paper width
    let mut lines: Vec<String> = Vec::with_capacity(20);

    // Header
    let separator = "─".repeat(w);
    lines.push(separator.clone());
    lines.push(center_text("KITCHEN ORDER", w));
    lines.push(separator.clone());
    lines.push(String::new());

    // Order number
    let order_str = display_number
        .map(|n| format!("#{}", n))
        .unwrap_or_else(|| "--".to_string());
    lines.push(format!("Order: {order_str}"));

    // Table number
    if let Some(table) = table_number
        && !table.is_empty()
    {
        lines.push(format!("Table: {table}"));
    }

    // Items count
    lines.push(format!("Items: {item_count}"));
    lines.push(String::new());
    lines.push(separator.clone());

    // Items
    lines.push("ITEMS:".to_string());
    for item_line in items_summary.split(", ") {
        let trimmed = item_line.trim();
        if !trimmed.is_empty() {
            lines.push(format!("  • {trimmed}"));
        }
    }
    lines.push(separator.clone());

    // Notes
    if !notes.is_empty() {
        lines.push(format!("NOTES: {notes}"));
        lines.push(separator.clone());
    }

    // Timestamp — pass through the received_at string as-is (already
    // in RFC3339 format from the database, no chrono dependency needed).
    lines.push(String::new());
    lines.push(format!("Received: {received_at}"));

    lines.push(separator.clone());
    lines.push(String::new());
    lines.push(center_text("<<< CUT HERE >>>", w));

    let text = lines.join("\n");

    // Build ESC/POS bytes
    let mut buf = Vec::with_capacity(text.len() + 64);
    buf.extend_from_slice(escpos::ESC_INIT);
    buf.extend_from_slice(escpos::LINE_SPACING_DEFAULT);
    buf.extend_from_slice(escpos::FONT_A);

    for line in text.lines() {
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(escpos::LF);
    }

    buf.extend_from_slice(&escpos::feed(3));
    buf.extend_from_slice(escpos::CUT_FULL);

    KdsChit { data: buf, text }
}

/// Center text by padding with spaces to the given width.
fn center_text(s: &str, width: usize) -> String {
    let len = s.len();
    if len >= width {
        s.to_owned()
    } else {
        let left = (width - len) / 2;
        let right = width - len - left;
        format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chit_contains_order_number() {
        let chit = format_kds_chit(Some(42), None, "Burger x2", 2, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains("#42"));
    }

    #[test]
    fn chit_contains_table_number() {
        let chit = format_kds_chit(Some(1), Some("T5"), "Fries", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains("Table: T5"));
    }

    #[test]
    fn chit_contains_items() {
        let chit = format_kds_chit(
            Some(7),
            None,
            "Steak x2, Salad x1",
            3,
            "",
            "2026-07-30T12:00:00Z",
        );
        assert!(chit.text.contains("Steak x2"));
        assert!(chit.text.contains("Salad x1"));
    }

    #[test]
    fn chit_contains_notes() {
        let chit = format_kds_chit(
            Some(3),
            None,
            "Pasta",
            1,
            "No cheese, gluten free",
            "2026-07-30T12:00:00Z",
        );
        assert!(chit.text.contains("NOTES:"));
        assert!(chit.text.contains("No cheese, gluten free"));
    }

    #[test]
    fn chit_without_table_number_omits_table_line() {
        let chit = format_kds_chit(Some(5), None, "Tea", 1, "", "2026-07-30T12:00:00Z");
        assert!(!chit.text.contains("Table:"));
    }

    #[test]
    fn chit_empty_notes_omits_notes_section() {
        let chit = format_kds_chit(Some(2), None, "Coffee", 1, "", "2026-07-30T12:00:00Z");
        assert!(!chit.text.contains("NOTES:"));
    }

    #[test]
    fn chit_contains_received_timestamp() {
        let chit = format_kds_chit(Some(1), None, "Item", 1, "", "2026-07-30T12:34:56Z");
        assert!(chit.text.contains("Received: 2026-07-30T12:34:56Z"));
    }

    #[test]
    fn chit_starts_with_esc_init() {
        let chit = format_kds_chit(Some(1), None, "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.data.starts_with(escpos::ESC_INIT));
    }

    #[test]
    fn chit_ends_with_full_cut() {
        let chit = format_kds_chit(Some(1), None, "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.data.ends_with(escpos::CUT_FULL));
    }

    #[test]
    fn chit_contains_separator() {
        let chit = format_kds_chit(Some(10), None, "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains('─'));
    }

    #[test]
    fn chit_contains_kitchen_order_heading() {
        let chit = format_kds_chit(Some(1), None, "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains("KITCHEN ORDER"));
    }

    #[test]
    fn chit_contains_item_count() {
        let chit = format_kds_chit(Some(1), None, "Item x3", 3, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains("Items: 3"));
    }

    #[test]
    fn center_text_pads_correctly() {
        let result = center_text("HELLO", 48);
        assert_eq!(result.len(), 48);
        assert_eq!(result.trim(), "HELLO");
    }

    #[test]
    fn center_text_long_string_not_padded() {
        let long = "A".repeat(60);
        let result = center_text(&long, 48);
        assert_eq!(result.len(), 60);
    }

    #[test]
    fn chit_handles_no_display_number() {
        let chit = format_kds_chit(None, None, "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(chit.text.contains("Order: --"));
    }

    #[test]
    fn chit_handles_empty_table_string() {
        let chit = format_kds_chit(Some(1), Some(""), "Item", 1, "", "2026-07-30T12:00:00Z");
        assert!(!chit.text.contains("Table:"));
    }

    #[test]
    fn chit_handles_unparseable_timestamp() {
        let chit = format_kds_chit(Some(1), None, "Item", 1, "", "not-a-timestamp");
        assert!(chit.text.contains("not-a-timestamp"));
    }

    #[test]
    fn chit_bullets_are_formatted() {
        let chit = format_kds_chit(
            Some(1),
            None,
            "Item A, Item B",
            2,
            "",
            "2026-07-30T12:00:00Z",
        );
        assert!(chit.text.contains("  • Item A"));
        assert!(chit.text.contains("  • Item B"));
    }
}
