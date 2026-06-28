# oz-hal

Hardware Abstraction Layer for OZ-POS. The seam between business logic (which wants "scan a barcode") and physical devices (which send bytes over USB, Bluetooth, or serial).

## Traits

Every device category has an `async` trait that returns `Result<T, HalError>`:

| Trait | File | Methods |
|-------|------|---------|
| `BarcodeScanner` | `traits/barcode.rs` | `connect`, `poll`, `cancel` |
| `ReceiptPrinter` | `traits/printer.rs` | `print_receipt`, `cut` |
| `CashDrawer` | `traits/cash_drawer.rs` | `open`, `is_open` |

Business code **never imports a specific driver** — it only sees traits via `DriverRegistry`.

## Drivers

| Driver | File | Status |
|--------|------|--------|
| `UsbHidBarcodeScanner` | `drivers/usb_scanner.rs` | **Real** — USB HID interrupt transfers, HID keycode → ASCII |
| `SerialBarcodeScanner` | `drivers/serial_scanner.rs` | **Stub** — reads serial port until terminator |
| `UsbReceiptPrinter` | `drivers/usb_printer.rs` | **Stub** — ESC/POS formatting over USB bulk OUT |
| `BtReceiptPrinter` | `drivers/bt_printer.rs` | **Stub** — Bluetooth SPP via virtual COM port |
| `TcpReceiptPrinter` | `drivers/tcp_printer.rs` | **Stub** — Wi-Fi/TCP raw port 9100 |
| `MockBarcodeScanner` | `drivers/mock.rs` | Programmable mock for tests |
| `MockReceiptPrinter` | `drivers/mock.rs` | Programmable mock for tests |
| `MockCashDrawer` | `drivers/mock.rs` | Programmable mock for tests |

### USB HID scanner (`UsbHidBarcodeScanner`)

Opens the USB device, claims the HID interface, and reads 8-byte HID keyboard reports via interrupt transfer. Converts HID usage IDs to ASCII (including Shift-key modifiers). Accumulates characters until Enter (0x28) terminates the scan.

**Supported devices** (by VID/PID): Honeywell Voyager 1450g/1900g, Datalogic Magellan/Gryphon/QuickScan, Zebra LI/DS series, and generic HID barcode scanners.

### Serial scanner (`SerialBarcodeScanner`)

Opens a serial port at the configured baud rate (default 9600) and reads until `\r` or `\n`. Detects common USB-serial adapters (FTDI, CH340, CP210x, Prolific) via VID/PID.

### USB printer (`UsbReceiptPrinter`)

Wraps receipt text in ESC/POS commands (init, font A, line feeds) and sends via USB bulk OUT endpoint. Supports ESC/POS full and partial cut.

**Supported printers** (by VID/PID): Epson TM-T20/T88VI/T70/TM-m30, Star SP700/TSP100/mC-Print3, Bixolon SRP-350/275.

### Bluetooth printer (`BtReceiptPrinter`)

Connects to a Bluetooth SPP (Serial Port Profile) printer via a virtual COM port. After the user pairs the printer with the OS, the driver opens the BT serial port and sends ESC/POS.

**Auto-discovery:** `DriverRegistry::discover()` calls `serial::probe_bluetooth()` to find paired BT serial ports and registers each as a `BtReceiptPrinter`.

**Supported printers:** Epson TM-m30 BT, Star SP700 BT, and any BT printer using SPP.

### TCP/network printer (`TcpReceiptPrinter`)

Connects to a network printer over raw TCP (port 9100). The user provides the printer's IP address or hostname via the setup wizard (not auto-discovered).

**Supported printers:** Epson TM-i series, Star mC-Print3 Wi-Fi, Bixolon SRP-350plus, and any printer that supports raw port 9100 printing.

## Transport layer

| Module | File | Purpose |
|--------|------|---------|
| `transport::usb` | `transport/usb.rs` | USB enumeration, VID/PID matching, device open/claim |
| `transport::serial` | `transport/serial.rs` | Serial port enumeration, BT port detection, open with POS settings |
| `transport::tcp` | `transport/tcp.rs` | Async TCP connection for network printers (port 9100) |

## Shared ESC/POS formatting

All printer drivers (`UsbReceiptPrinter`, `BtReceiptPrinter`, `TcpReceiptPrinter`) share a single ESC/POS formatting module at `drivers::escpos`. It provides:

- `escpos::format_receipt(body)` — wraps plain text in init + font + line feeds
- `escpos::CUT_FULL` / `escpos::CUT_PARTIAL` — cut command bytes
- `escpos::ESC_INIT`, `escpos::LINE_SPACING_DEFAULT` — common init sequences

Both modules are used by `DriverRegistry::discover()` at startup.

## Registry

`DriverRegistry` holds `Arc<dyn Trait>` per device category behind `RwLock`. Call `discover()` to probe for all hardware:

```rust
let registry = DriverRegistry::default();

// Auto-discover USB / serial / BT hardware
registry.discover().await;

// Manually register a TCP network printer (user-configured)
let info = DeviceInfo::new("Epson", "TM-i Series", "192.168.1.100");
registry.register_tcp_printer("printer:tm-counter", "192.168.1.100", info).await;

if let Some(scanner) = registry.scanner("scanner:usb:<serial>").await {
    let mut scanner = scanner.connect().await?;
    let barcode = scanner.poll(5000).await?;
}
```

## Mocks

Every trait has a programmable mock in `drivers/mock.rs`. Tests push inputs into a queue and assert call counters — no physical hardware needed.

```rust
let scanner = MockBarcodeScanner::new();
scanner.push(Barcode::new("ABC123"));
```

## Conventions

- `unsafe` is allowed (drivers may need FFI). Wrap every `unsafe` block in a `// SAFETY:` comment.
- Every trait must have a mock. Tests use mocks, never real hardware.
- Mocks are `Send + Sync + Clone` with `AtomicUsize` counters for assertions.
- No `unwrap()` in driver code. Map errors to `HalError` at the trait boundary.
- Use `tokio::sync::Mutex` (not `std::sync::Mutex`) when holding across `.await`.
- Wrap blocking I/O in `tokio::task::spawn_blocking`.

See the `hal-drivers` skill under `.agents/skills/` for the full conventions.
