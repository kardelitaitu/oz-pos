//! ESC/POS command constants and receipt formatting helpers.
//!
//! Shared across all printer drivers ([`UsbReceiptPrinter`],
//! [`BtReceiptPrinter`], [`TcpReceiptPrinter`]) and the receipt
//! formatter ([`super::receipt`]).

/// Initialize printer.
pub const ESC_INIT: &[u8] = &[0x1B, 0x40];
/// Print and carriage return.
pub const LF: &[u8] = &[0x0A];
/// Cut paper (full cut).
pub const CUT_FULL: &[u8] = &[0x1D, 0x56, 0x00];
/// Cut paper (partial cut).
pub const CUT_PARTIAL: &[u8] = &[0x1D, 0x56, 0x01];

/// Select character font A (12×24).
pub const FONT_A: &[u8] = &[0x1B, 0x4D, 0x00];
/// Select character font B (9×17).
#[allow(dead_code)]
pub const FONT_B: &[u8] = &[0x1B, 0x4D, 0x01];

/// Set line spacing to default (30 dots).
pub const LINE_SPACING_DEFAULT: &[u8] = &[0x1B, 0x32];

// ── Alignment ────────────────────────────────────────────
/// Left-align subsequent text.
pub const ALIGN_LEFT: &[u8] = &[0x1B, 0x61, 0x00];
/// Centre-align subsequent text.
pub const ALIGN_CENTER: &[u8] = &[0x1B, 0x61, 0x01];
/// Right-align subsequent text.
pub const ALIGN_RIGHT: &[u8] = &[0x1B, 0x61, 0x02];

// ── Font decoration ──────────────────────────────────────
/// Enable bold/emphasised mode.
pub const BOLD_ON: &[u8] = &[0x1B, 0x45, 0x01];
/// Disable bold/emphasised mode.
pub const BOLD_OFF: &[u8] = &[0x1B, 0x45, 0x00];

/// Underline on (1-dot thickness).
#[allow(dead_code)]
pub const UNDERLINE_ON: &[u8] = &[0x1B, 0x2D, 0x01];
/// Underline off.
#[allow(dead_code)]
pub const UNDERLINE_OFF: &[u8] = &[0x1B, 0x2D, 0x00];

// ── Character size (GS ! n) ──────────────────────────────
/// Normal 1×1 size.
#[allow(dead_code)]
pub const SIZE_NORMAL: &[u8] = &[0x1D, 0x21, 0x00];
/// Double height (2×1).
#[allow(dead_code)]
pub const DBL_HEIGHT: &[u8] = &[0x1D, 0x21, 0x01];
/// Double width (1×2).
#[allow(dead_code)]
pub const DBL_WIDTH: &[u8] = &[0x1D, 0x21, 0x10];
/// Double height + width (2×2).
#[allow(dead_code)]
pub const DBL_BOTH: &[u8] = &[0x1D, 0x21, 0x11];

// ── Feed / spacing ───────────────────────────────────────
/// Feed n lines (ESC d n). Call with the desired count.
pub fn feed(n: u8) -> Vec<u8> {
    vec![0x1B, 0x64, n]
}

// ── Legacy formatter ─────────────────────────────────────

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

    #[test]
    fn alignment_commands_are_correct() {
        assert_eq!(ALIGN_LEFT, &[0x1B, 0x61, 0x00]);
        assert_eq!(ALIGN_CENTER, &[0x1B, 0x61, 0x01]);
        assert_eq!(ALIGN_RIGHT, &[0x1B, 0x61, 0x02]);
    }

    #[test]
    fn bold_commands_are_correct() {
        assert_eq!(BOLD_ON, &[0x1B, 0x45, 0x01]);
        assert_eq!(BOLD_OFF, &[0x1B, 0x45, 0x00]);
    }

    #[test]
    fn size_commands_are_correct() {
        assert_eq!(SIZE_NORMAL, &[0x1D, 0x21, 0x00]);
        assert_eq!(DBL_HEIGHT, &[0x1D, 0x21, 0x01]);
        assert_eq!(DBL_WIDTH, &[0x1D, 0x21, 0x10]);
        assert_eq!(DBL_BOTH, &[0x1D, 0x21, 0x11]);
    }

    #[test]
    fn feed_n_produces_correct_bytes() {
        assert_eq!(feed(3), &[0x1B, 0x64, 3]);
        assert_eq!(feed(0), &[0x1B, 0x64, 0]);
    }
}
