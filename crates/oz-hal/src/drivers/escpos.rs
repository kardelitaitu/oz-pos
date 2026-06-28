//! ESC/POS command constants and receipt formatting helpers.
//!
//! Shared across all printer drivers ([`UsbReceiptPrinter`],
//! [`BtReceiptPrinter`], [`TcpReceiptPrinter`]) so there's a single
//! source of truth for the ESC/POS byte sequences.

/// Initialize printer.
pub const ESC_INIT: &[u8] = &[0x1B, 0x40];
/// Print and carriage return.
pub const LF: &[u8] = &[0x0A];
/// Cut paper (full cut).
pub const CUT_FULL: &[u8] = &[0x1D, 0x56, 0x00];
/// Cut paper (partial cut).
pub const CUT_PARTIAL: &[u8] = &[0x1D, 0x56, 0x01];
/// Select character font A (12×24).
#[allow(dead_code)]
pub const FONT_A: &[u8] = &[0x1B, 0x4D, 0x00];
/// Select character font B (9×17).
#[allow(dead_code)]
pub const FONT_B: &[u8] = &[0x1B, 0x4D, 0x01];
/// Set line spacing to default (30 dots).
pub const LINE_SPACING_DEFAULT: &[u8] = &[0x1B, 0x32];

/// Build an ESC/POS byte buffer from a plain-text receipt body.
pub fn format_receipt(body: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(body.len() + 64);

    buf.extend_from_slice(ESC_INIT);
    buf.extend_from_slice(LINE_SPACING_DEFAULT);
    buf.extend_from_slice(FONT_A);

    for line in body.lines() {
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(LF);
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_receipt_starts_with_init() {
        let data = format_receipt("Hello\nWorld");
        assert!(data.starts_with(ESC_INIT), "missing ESC @ init");
    }

    #[test]
    fn formatted_receipt_contains_body_text() {
        let data = format_receipt("Hello\nWorld");
        assert!(
            data.windows(b"Hello".len()).any(|w| w == b"Hello"),
            "missing body text"
        );
    }

    #[test]
    fn formatted_receipt_has_line_feeds() {
        let data = format_receipt("Hello\nWorld");
        assert!(
            data.windows(LF.len()).any(|w| w == LF),
            "missing line feeds"
        );
    }

    #[test]
    fn cut_commands_are_correct() {
        assert_eq!(CUT_FULL, &[0x1D, 0x56, 0x00]);
        assert_eq!(CUT_PARTIAL, &[0x1D, 0x56, 0x01]);
    }
}
