# oz-hal

Hardware Abstraction Layer for OZ-POS. The seam between business logic (which wants "scan a barcode") and physical devices (which send bytes over USB, Bluetooth, or serial).

## Public API

- [`HalError`](src/error.rs) — `thiserror`-based error returned by every HAL trait method. `#[non_exhaustive]`.

## Planned modules (scaffold in place)

- `traits` — `BarcodeScanner`, `ReceiptPrinter`, `NfcReader`, `PaymentTerminal`, `CashDrawer`.
- `drivers` — vendor-specific implementations (Honeywell, Star, ACR122U, IDTech).
- `drivers::mock` — mandatory mocks for every trait (test-only).
- `registry` — `DriverRegistry` for runtime discovery and plug-and-play.
- `transport` — USB, Bluetooth, serial backends.

## Conventions

- `unsafe` is allowed (drivers may need FFI). Wrap every `unsafe` block in a `// SAFETY:` comment.
- Every trait must have a mock. Tests use mocks, never real hardware.
- Mocks are `Send + Sync + Clone` with `AtomicUsize` counters for assertions.

See the `hal-drivers` skill under `.agents/skills/` for the full conventions.
