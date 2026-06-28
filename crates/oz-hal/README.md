# oz-hal

Hardware Abstraction Layer — the seam between business logic and physical devices (USB, Bluetooth, serial, TCP).

## Traits

| Trait | File | Methods |
|-------|------|---------|
| `BarcodeScanner` | `traits/barcode.rs` | `connect`, `poll`, `cancel` |
| `ReceiptPrinter` | `traits/printer.rs` | `print_receipt`, `print_raw`, `cut` |
| `CashDrawer` | `traits/cash_drawer.rs` | `open`, `is_open` |

Business code never imports a specific driver — only traits via `DriverRegistry`.

## Drivers

| Driver | File | Status |
|--------|------|--------|
| `UsbHidBarcodeScanner` | `drivers/usb_scanner.rs` | Real — USB HID interrupt + keycode→ASCII |
| `SerialBarcodeScanner` | `drivers/serial_scanner.rs` | Stub |
| `UsbReceiptPrinter` | `drivers/usb_printer.rs` | Stub |
| `BtReceiptPrinter` | `drivers/bt_printer.rs` | Stub |
| `TcpReceiptPrinter` | `drivers/tcp_printer.rs` | Stub |
| `MockBarcodeScanner` | `drivers/mock.rs` | Programmable mock |
| `MockReceiptPrinter` | `drivers/mock.rs` | Programmable mock |
| `MockCashDrawer` | `drivers/mock.rs` | Programmable mock |

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

> last audited 28-06-26 by docs-auditor
