# oz-hal

<!-- Audit stamp: 2026-08-31 · docs-auditor · status: ACCURATE (3 findings repaired) · F1: removed dead `Scanner`/`drivers/scanner.rs` row (file deleted at HEAD; scanners covered by usb/bt/serial_scanner) · F2: added `EdcTerminal` trait row (traits/edc.rs — status/authorize/capture/sale/refund/void/print_receipt/device_info; re-exported with EdcPaymentResult/TerminalStatus) · F3: added EDC drivers WiredEdcTerminal (drivers/edc/wired.rs), WirelessEdcTerminal (drivers/edc/wireless.rs), protocol codecs Ingenico/PAX/Verifone (drivers/edc/protocol/), and MockEdcTerminal (drivers/mock.rs) · verified accurate: remaining traits (barcode/printer/cash_drawer/customer_display/weight_scale), drivers (usb/bt/serial/tcp printer, drawer, serial_display, scale, kds_chit), mocks, escpos consts + format_receipt, receipt format_sales_receipt/SalesReceipt/ReceiptConfig, DriverRegistry methods; unsafe confined to lib.rs with SAFETY comment. NOTE: EdcTerminal is mid-migration per lib.rs doc — trait + drivers exist at HEAD, registry wiring not yet claimed -->

Hardware Abstraction Layer — the seam between business logic and physical devices (USB, Bluetooth, serial, TCP).

## Traits

| Trait | File | Methods |
|-------|------|---------|
| `BarcodeScanner` | `traits/barcode.rs` | `connect`, `poll`, `cancel` |
| `ReceiptPrinter` | `traits/printer.rs` | `print_receipt`, `print_raw`, `cut` |
| `CashDrawer` | `traits/cash_drawer.rs` | `open`, `is_open` |
| `CustomerDisplay` | `traits/customer_display.rs` | Pole/line display for customer-facing screen |
| `WeightScale` | `traits/weight_scale.rs` | `WeightScale`, `WeightReading` — re-exported at crate root |
| `EdcTerminal` | `traits/edc.rs` | `status`, `authorize`, `capture`, `sale`, `refund`, `void`, `print_receipt`, `device_info` — card-payment terminals; re-exported with `EdcPaymentResult`/`TerminalStatus` |

Business code never imports a specific driver — only traits via `DriverRegistry`.

### Public modules

| Module | Contents |
|--------|----------|
| `error` | `HalError`, `HalErrorKind` — `thiserror`-based error types |
| `transport` | USB/serial/BT/TCP transport abstractions |
| `types` | `Barcode`, `BarcodeSymbology`, `DeviceInfo` |
| `registry` | `DriverRegistry` — auto-discovery and manual registration |

## Drivers

| Driver | File | Status |
|--------|------|--------|
| `UsbHidBarcodeScanner` | `drivers/usb_scanner.rs` | Real — USB HID interrupt + keycode→ASCII |
| `BtBarcodeScanner` | `drivers/bt_scanner.rs` | Stub |
| `SerialBarcodeScanner` | `drivers/serial_scanner.rs` | Stub |
| `UsbReceiptPrinter` | `drivers/usb_printer.rs` | Stub |
| `BtReceiptPrinter` | `drivers/bt_printer.rs` | Stub |
| `TcpReceiptPrinter` | `drivers/tcp_printer.rs` | Stub |
| `CashDrawer` | `drivers/drawer.rs` | Cash drawer driver |
| `SerialCustomerDisplay` | `drivers/serial_display.rs` | Stub |
| `HidWeightScale` | `drivers/scale.rs` | USB HID weight scale driver |
| `KdsChit` | `drivers/kds_chit.rs` | KDS chit printer |
| `WiredEdcTerminal` | `drivers/edc/wired.rs` | Real — wired card terminal; protocol codecs (Ingenico/PAX/Verifone) in `drivers/edc/protocol/` |
| `WirelessEdcTerminal` | `drivers/edc/wireless.rs` | Real — wireless card terminal; shares the `drivers/edc/protocol/` codecs |
| `MockBarcodeScanner` | `drivers/mock.rs` | Programmable mock |
| `MockReceiptPrinter` | `drivers/mock.rs` | Programmable mock |
| `MockCashDrawer` | `drivers/mock.rs` | Programmable mock |
| `MockEdcTerminal` | `drivers/mock.rs` | Programmable mock |

## ESC/POS & receipt formatting

All printer drivers share a single ESC/POS module at `drivers::escpos`:

- `escpos::format_receipt(body)` — wraps text in init + font + commands
- `escpos::CUT_FULL` / `CUT_PARTIAL` — cut command bytes
- `escpos::ALIGN_CENTER`, `BOLD_ON`, `BOLD_OFF` — formatting constants
- Receipt formatting lives in `drivers::receipt`: `format_sales_receipt()` builds a full ESC/POS buffer from structured `SalesReceipt` + `ReceiptConfig` data.

## Registry

`DriverRegistry` holds `Arc<dyn Trait>` per device behind `RwLock`:

```rust
let registry = DriverRegistry::default();
registry.discover().await;                               // auto-probe USB/serial/BT
registry.register_tcp_printer("printer:tm-counter", "192.168.1.100").await; // manual
if let Some(scanner) = registry.scanner("scanner:usb:<serial>").await {
    let barcode = scanner.connect().await?.poll(5000).await?;
}
```

## Mocks

Every trait has a programmable mock in `drivers/mock.rs`:

```rust
let scanner = MockBarcodeScanner::new();
scanner.push(Barcode::new("ABC123"));
```

## Conventions

- `unsafe` allowed with `// SAFETY:` comment.
- Every trait must have a mock (`Send + Sync + Clone` with `AtomicUsize` counters).
- No `unwrap()` in driver code — map errors to `HalError` at the trait boundary.
- Wrap blocking I/O in `tokio::task::spawn_blocking`.

> last audited 31-08-26 by docs-auditor
