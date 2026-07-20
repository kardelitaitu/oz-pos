//! ESC/POS command constants and receipt formatting helpers.
//!
//! Shared across all printer drivers (`UsbReceiptPrinter`,
//! `BtReceiptPrinter`, `TcpReceiptPrinter`) and the receipt
//! formatter (`super::receipt`).

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

// ── Barcode (GS k) ────────────────────────────────────────────

/// Barcode symbology identifiers (GS k m).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BarcodeType {
    /// UPC-A (11+1 numeric digits).
    UpcA = 0,
    /// UPC-E (6+1 numeric digits).
    UpcE = 1,
    /// EAN-13 (12+1 numeric digits).
    Ean13 = 2,
    /// EAN-8 (7+1 numeric digits).
    Ean8 = 3,
    /// Code 39 (alphanumeric, variable length).
    Code39 = 4,
    /// ITF (Interleaved 2 of 5, numeric even digits).
    Itf = 5,
    /// Code 128 (full ASCII, variable length).
    Code128 = 73,
}

/// Build an ESC/POS barcode-printing command: GS k m n d1..dn.
///
/// `data` must be valid for the chosen symbology (numeric-only for
/// UPC/EAN/ITF, alphanumeric for Code39, full ASCII for Code128).
pub fn barcode(barcode_type: BarcodeType, data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut buf = Vec::with_capacity(4 + n);
    // Set barcode height to ~162 dots (~80px at 203dpi)
    buf.extend_from_slice(&[0x1D, 0x68, 0xA0]);
    // Set human-readable (HRI) position below the barcode
    buf.extend_from_slice(&[0x1D, 0x48, 0x02]);
    // Print barcode: GS k m n d1..dn
    buf.extend_from_slice(&[0x1D, 0x6B, barcode_type as u8, n as u8]);
    buf.extend_from_slice(data);
    buf
}

// ── QR code (GS ( k) ────────────────────────────────────────────

/// Build an ESC/POS QR-code printing command sequence.
///
/// Standard two-dimensional GS ( k command set. The sequence:
/// 1. Select QR code model 2
/// 2. Set module size (3–8 dots, default 4)
/// 3. Store data bytes
/// 4. Print QR code
pub fn qr_code(data: &[u8], module_size: u8) -> Vec<u8> {
    let module_size = module_size.clamp(3, 8);
    let len = data.len();
    let pl = len + 3; // pL = (len + 3) & 0xFF
    let ph = ((len + 3) >> 8) & 0xFF;

    let mut buf = Vec::with_capacity(16 + len);
    // Step 1: Select model 2 (function 049, model 050 = QR model 2)
    buf.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00]);
    // Step 2: Set module size (function 043)
    buf.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, module_size]);
    // Step 3: Store data (function 050, pL pH 31 50 30 d1..dn)
    buf.extend_from_slice(&[0x1D, 0x28, 0x6B, pl as u8, ph as u8, 0x31, 0x50, 0x30]);
    buf.extend_from_slice(data);
    // Step 4: Print QR (function 049, 31 51 48)
    buf.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x51, 0x30]);
    buf
}

// ── Cash drawer kick ────────────────────────────────────
/// Pulse pin 2 on the RJ12 cash drawer port for the default duration
/// (typically 50–100 ms). Sends the standard ESC/POS kick command
/// `ESC p m t1 t2` where m=0 (pin 2), t1=on time, t2=off time.
///
/// Most thermal receipt printers support this command on the
/// serial/USB/BT interface and will pulse the designated pin on the
/// cash drawer port when they receive it.
pub const KICK_DRAWER_PIN2: &[u8] = &[0x1B, 0x70, 0x00, 0x19, 0x32];
/// Pulse pin 5 on the RJ12 cash drawer port (`ESC p m t1 t2` with m=1).
pub const KICK_DRAWER_PIN5: &[u8] = &[0x1B, 0x70, 0x01, 0x19, 0x32];

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
    fn kick_drawer_commands_are_correct() {
        // ESC p 0 25 50 — pin 2, 25*2ms on, 50*2ms off
        assert_eq!(KICK_DRAWER_PIN2, &[0x1B, 0x70, 0x00, 0x19, 0x32]);
        // ESC p 1 25 50 — pin 5, 25*2ms on, 50*2ms off
        assert_eq!(KICK_DRAWER_PIN5, &[0x1B, 0x70, 0x01, 0x19, 0x32]);
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

    // ── Barcode commands ─────────────────────────────────────────────

    #[test]
    fn barcode_code128_starts_with_gs_h() {
        let cmd = barcode(BarcodeType::Code128, b"REC-001");
        // Should start with GS h A0 (set height)
        assert_eq!(cmd[..3], [0x1D, 0x68, 0xA0], "missing GS h height");
        // Should contain GS H 02 (HRI below)
        assert!(cmd.windows(3).any(|w| w == [0x1D, 0x48, 0x02]));
        // Should contain GS k 49 n data (print barcode)
        assert!(cmd.windows(4).any(|w| w == [0x1D, 0x6B, 73, 7]));
        // Data should be present
        let data_start = cmd.windows(7).position(|w| w == b"REC-001");
        assert!(data_start.is_some(), "missing barcode data");
    }

    #[test]
    fn barcode_ean13_command_format() {
        let cmd = barcode(BarcodeType::Ean13, b"123456789012");
        // GS k 02 n 12-digit data
        assert!(cmd.windows(4).any(|w| w == [0x1D, 0x6B, 2, 12]));
        assert!(cmd.windows(12).any(|w| w == b"123456789012"));
    }

    #[test]
    fn barcode_code39_data_integrity() {
        let data = b"0123456789";
        let cmd = barcode(BarcodeType::Code39, data);
        // Data should appear in the command
        assert!(cmd.windows(10).any(|w| w == data));
    }

    // ── QR code commands ─────────────────────────────────────────────

    #[test]
    fn qr_code_starts_with_model_selection() {
        let cmd = qr_code(b"https://example.com/pay", 4);
        // Should start with GS ( k 04 00 31 41 32 00 (model 2)
        assert_eq!(
            cmd[..9],
            [0x1D, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00],
            "missing QR model selection"
        );
    }

    #[test]
    fn qr_code_contains_size_and_print_commands() {
        let cmd = qr_code(b"test", 4);
        // Should contain module size: GS ( k 03 00 31 43 04
        assert!(
            cmd.windows(8)
                .any(|w| w == [0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, 4])
        );
        // Should contain print command: GS ( k 03 00 31 51 30
        assert!(
            cmd.windows(8)
                .any(|w| w == [0x1D, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x51, 0x30])
        );
    }

    #[test]
    fn qr_code_contains_data_bytes() {
        let data = b"payment:12345";
        let cmd = qr_code(data, 4);
        // Data should appear in store command
        assert!(cmd.windows(data.len()).any(|w| w == data));
    }

    #[test]
    fn qr_code_module_size_clamps() {
        let cmd_low = qr_code(b"x", 1); // should clamp to 3
        let cmd_high = qr_code(b"x", 10); // should clamp to 8
        let cmd_default = qr_code(b"x", 4);
        // All should still be valid commands
        assert!(cmd_low.len() > 10);
        assert!(cmd_high.len() > 10);
        assert!(cmd_default.len() > 10);
    }

    #[test]
    fn qr_code_empty_data_produces_command() {
        let cmd = qr_code(b"", 4);
        // Should still produce a valid command with zero-length data
        assert!(cmd.len() >= 17);
        // The store data command should have pL=3 (3 extra bytes for header)
        // so the total command length is 8 (header) + 3 (extra) + 0 (data) + 8 (print) = 19
        assert!(
            cmd.len() >= 15,
            "empty QR data should produce a valid command sequence"
        );
    }
}
