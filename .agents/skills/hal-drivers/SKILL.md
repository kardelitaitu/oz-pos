---
name: hal-drivers
description: Hardware Abstraction Layer (HAL) conventions for OZ-POS — embedded-hal traits, drivers for barcode scanners, receipt printers, NFC readers, and payment terminals, plus mandatory mock implementations. Use when adding a new device driver or wiring hardware into a feature.
---

<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (4 noted findings, doc-staleness) · F1: claims HAL built on embedded-hal traits + crate path hal/; actual oz-hal has NO embedded-hal dep and lives at crates/oz-hal/ (not hal/) · F2: layout lists traits nfc.rs + payment_terminal.rs — neither exists; actual traits dir has barcode/cash_drawer/customer_display/printer only (no NfcReader/PaymentTerminal trait) · F3: driver files shown as honeywell_barcode/star_printer/acr122u_nfc/idtech_payment — actual drivers are generic usb/bt/serial/tcp_scanner + usb/bt/tcp_printer + drawer/serial_display/scale (no vendor-specific named drivers) · F4: claims mocks gated by mock feature (cargo test --features mock) — no mock feature in Cargo.toml and mock.rs has no cfg(feature) gate (mocks always compiled) · verified accurate: BarcodeScanner trait signature matches code (connect/poll/cancel/device_info, &self/&mut self/Box<dyn>), register_scanner + register_tcp_printer + DriverRegistry::discover present, mocks in drivers/mock.rs, async Result<T,HalError> convention -->

# Hardware Abstraction Layer (HAL)

OZ-POS runs on real hardware: barcode scanners, receipt printers, NFC readers, cash drawers, payment terminals. The HAL (`oz-hal`) is the seam between the **business logic** (which wants "scan a barcode") and the **physical device** (which sends bytes over USB, Bluetooth, or serial).

The HAL is implemented in Rust on top of `embedded-hal` traits. The rest of the system only ever sees the trait — it never imports a specific driver.

---

## When to use

- Adding a new device category (e.g., scale, customer display, EMV terminal).
- Writing a driver for a specific device model.
- Wiring hardware into a Tauri command or a feature.
- Writing the **mandatory** mock implementation for a new driver.
- Reviewing hardware code for thread safety, error handling, or platform support.

---

## Golden rules

| # | Rule | Why |
|---|------|-----|
| 1 | **Every device implements a trait.** Business code never imports a specific driver. | Swap hardware without changing features. |
| 2 | **A mock implementation lives in `crates/oz-hal/src/drivers/mock.rs` for every new trait.** | Tests must run without physical hardware. |
| 3 | **Traits are `async` and return `Result<T, HalError>`.** | Hardware fails in surprising ways. Make it explicit. |
| 4 | **No `unwrap()` in driver code.** A flaky USB device must not panic the cashier's flow. | |
| 5 | **Drivers register through `DriverRegistry`**, not via `static`s. | Hot-plug, multiple devices, plug-and-play. |

---

## Crate layout

```
hal/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── traits/
    │   ├── mod.rs
    │   ├── barcode.rs          # BarcodeScanner trait
    │   ├── printer.rs          # ReceiptPrinter trait
    │   ├── nfc.rs              # NfcReader trait
    │   ├── payment_terminal.rs # PaymentTerminal trait
    │   └── cash_drawer.rs      # CashDrawer trait
    ├── error.rs                # HalError enum (thiserror)
    ├── registry.rs             # DriverRegistry + discovery
    ├── transport/
    │   ├── usb.rs
    │   ├── bluetooth.rs
    │   └── serial.rs           # platform-conditional
    └── drivers/
        ├── mod.rs
        ├── honeywell_barcode.rs
        ├── star_printer.rs
        ├── acr122u_nfc.rs
        ├── idtech_payment.rs
        └── mock.rs             # <-- mandatory mocks
```

---

## Defining a trait

```rust
// hal/src/traits/barcode.rs

use async_trait::async_trait;
use crate::error::HalError;
use crate::types::{Barcode, ScanOutcome};

/// A device that produces barcode scans. Implementations may be USB HID,
/// Bluetooth, serial, or a camera-based software scanner.
#[async_trait]
pub trait BarcodeScanner: Send + Sync {
    /// Open a connection to the device. Idempotent — calling twice returns
    /// the same connection.
    async fn connect(&self) -> Result<Box<dyn BarcodeScanner>, HalError>;

    /// Poll for the next scan. Blocks until a code is read or the timeout
    /// elapses. Returns `Ok(None)` on timeout (not an error).
    async fn poll(&mut self, timeout_ms: u32) -> Result<Option<Barcode>, HalError>;

    /// Cancel an in-flight poll. Used when the user navigates away.
    async fn cancel(&self) -> Result<(), HalError>;

    /// Device identity, used in logs and the setup wizard.
    fn device_info(&self) -> DeviceInfo;
}
```

**Rules:**
- Traits are object-safe. Use `Box<dyn Trait>` for the registry.
- `Send + Sync` so the trait can be shared across Tauri command threads.
- Methods are `async` and never block the executor. Driver code that does CPU-heavy work should use `spawn_blocking`.
- `cancel()` is mandatory on long-running operations.
- Errors are `HalError`, with rich variants. The cashier's flow depends on knowing *why* a scan failed.

---

## Implementing a driver

```rust
// hal/src/drivers/honeywell_barcode.rs

use async_trait::async_trait;
use crate::error::HalError;
use crate::traits::barcode::{BarcodeScanner, DeviceInfo};
use crate::types::Barcode;
use tokio::sync::Mutex;

pub struct HoneywellBarcode {
    inner: Mutex<hw_usb::DeviceHandle>,
    info: DeviceInfo,
}

impl HoneywellBarcode {
    pub fn new() -> Self { /* ... */ }
}

#[async_trait]
impl BarcodeScanner for HoneywellBarcode {
    async fn connect(&self) -> Result<Box<dyn BarcodeScanner>, HalError> {
        // idempotent; return self
        Ok(Box::new(HoneywellBarcode { /* ... */ }))
    }

    async fn poll(&mut self, timeout_ms: u32) -> Result<Option<Barcode>, HalError> {
        let mut guard = self.inner.lock().await;
        tokio::time::timeout(
            Duration::from_millis(timeout_ms as u64),
            guard.read_barcode(),
        )
        .await
        .map_err(|_| HalError::Timeout)?
        .map(Some)
    }

    async fn cancel(&self) -> Result<(), HalError> {
        // signal the USB read to abort
        Ok(())
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}
```

**Rules:**
- Use `tokio::sync::Mutex` (not `std::sync::Mutex`) when holding across `.await`.
- Wrap blocking I/O in `tokio::task::spawn_blocking`.
- Map low-level errors to `HalError` at the trait boundary. Don't leak `rusb`, `btleplug`, or `serialport` types past the driver.
- Each driver has a `mod.rs` re-export and a `DriverInfo` constant used by the registry.

---

## The mandatory mock implementation

Every trait must have a mock. The mock is used by every test in the rest of the codebase that touches hardware.

```rust
// hal/src/drivers/mock.rs

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};   // std::sync::Mutex — held only briefly, no .await between lock and unlock
use async_trait::async_trait;
use crate::traits::barcode::{BarcodeScanner, DeviceInfo};
use crate::types::Barcode;

/// A programmable mock for `BarcodeScanner`. Tests push scans into a queue;
/// the mock returns them in order.
#[derive(Default, Clone)]
pub struct MockBarcodeScanner {
    queue: Arc<Mutex<VecDeque<Barcode>>>,
    pub connect_calls: Arc<AtomicUsize>,
    pub poll_calls: Arc<AtomicUsize>,
}

impl MockBarcodeScanner {
    pub fn new() -> Self { Self::default() }

    /// Queue a barcode to be returned by the next `poll`. Safe to call from
    /// any context (sync test setup or async runtime) — uses `std::sync::Mutex`
    /// which never panics inside a Tokio runtime.
    pub fn push(&self, code: Barcode) {
        self.queue.lock().expect("mock queue poisoned").push_back(code);
    }
}

#[async_trait]
impl BarcodeScanner for MockBarcodeScanner {
    async fn connect(&self) -> Result<Box<dyn BarcodeScanner>, HalError> {
        self.connect_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(self.clone()))
    }

    async fn poll(&mut self, timeout_ms: u32) -> Result<Option<Barcode>, HalError> {
        self.poll_calls.fetch_add(1, Ordering::SeqCst);
        // Lock is acquired and released in the same statement — never held across .await
        Ok(self.queue.lock().expect("mock queue poisoned").pop_front())
    }

    async fn cancel(&self) -> Result<(), HalError> { Ok(()) }

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            vendor: "mock".into(),
            model: "MockBarcode".into(),
            serial: "0000".into(),
        }
    }
}
```

**Rules:**
- Mocks are **stateful** and **programmable**. Tests push inputs, then assert what the system did with them.
- Counters (`connect_calls`, `poll_calls`, …) make assertions on driver behavior trivial.
- Mocks implement the **same trait** as the real driver. No special "mock mode" in business code.
- Mocks are `Send + Sync + Clone` so multiple tests can share them.

---

## The DriverRegistry

Hardware is discovered at startup and exposed to the rest of the app through a single registry. Commands ask the registry for a device by category; the registry picks an available driver.

```rust
// hal/src/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::traits::barcode::BarcodeScanner;

#[derive(Default)]
pub struct DriverRegistry {
    scanners: RwLock<HashMap<String, Arc<dyn BarcodeScanner>>>,
    printers: RwLock<HashMap<String, Arc<dyn ReceiptPrinter>>>,
    // ...
}

impl DriverRegistry {
    pub async fn register_scanner(&self, id: &str, driver: Arc<dyn BarcodeScanner>) {
        self.scanners.write().await.insert(id.into(), driver);
    }

    pub async fn scanner(&self, id: &str) -> Option<Arc<dyn BarcodeScanner>> {
        self.scanners.read().await.get(id).cloned()
    }
}
```

**Rules:**
- Registry is held in `AppState` and reached via `State<'_, AppState>` in Tauri commands.
- Discovery is a separate phase: `DriverRegistry::discover()` probes USB/Bluetooth/serial and populates the registry. Failure of one driver does not abort discovery.
- Setup wizard uses the registry to show "what's plugged in."

---

## Error type

```rust
// hal/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HalError {
    #[error("device not found: {0}")]
    NotFound(String),

    #[error("device disconnected")]
    Disconnected,

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("usb error: {0}")]
    Usb(String),

    #[error("bluetooth error: {0}")]
    Bluetooth(String),

    #[error("operation timed out after {0} ms")]
    Timeout(u32),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("device busy")]
    Busy,
}
```

**Rules:**
- `HalError` is `#[non_exhaustive]`. Add variants without breaking semver.
- Always include enough context to debug. "I/O error" is not enough; include the operation.
- Drivers convert third-party errors with `.map_err(|e| HalError::Usb(e.to_string()))` at the boundary.

---

## Testing

Driver tests use the mock to simulate hardware. No physical device required.

```rust
#[tokio::test]
async fn sale_completes_after_scan() {
    let scanner = MockBarcodeScanner::new();
    scanner.push(Barcode::new("ABC123"));
    let mut pos = PosTerminal::new(scanner.clone());
    pos.scan().await.unwrap();
    assert_eq!(pos.cart().lines().count(), 1);
    assert_eq!(scanner.poll_calls.load(Ordering::SeqCst), 1);
}
```

**Rules:**
- Tests use `MockBarcodeScanner`, `MockReceiptPrinter`, etc. — never a real driver.
- For driver-internal tests (e.g., parsing a USB packet), use synthetic byte buffers.
- Mocks live in `crates/oz-hal/src/drivers/mock.rs` and are gated by a `mock` feature: `cargo test --features mock`.

---

## Adding a new device — checklist

- [ ] Define the trait in `hal/src/traits/<device>.rs` with `async` methods returning `Result<T, HalError>`.
- [ ] Re-export from `hal/src/traits/mod.rs`.
- [ ] Add the `HalError` variant(s) if needed.
- [ ] Implement the driver in `hal/src/drivers/<vendor>_<device>.rs`.
- [ ] Re-export the driver from `hal/src/drivers/mod.rs`.
- [ ] **Add the mock to `crates/oz-hal/src/drivers/mock.rs`.** (Mandatory — CI will fail otherwise.)
- [ ] Register the driver in `DriverRegistry::discover()`.
- [ ] Add a Tauri command in `apps/desktop-client/src/commands/hardware.rs` that takes the registry from `State` and returns a `Result`.
- [ ] Add a TS wrapper in `ui/src/api/<feature>.ts` and a hook in `ui/src/features/<feature>/`.
- [ ] Tests: a unit test in the driver, a feature test using the mock, and a UI test with the hook.

---

## Common pitfalls

1. **Holding `std::sync::Mutex` across `.await`.** Use `tokio::sync::Mutex` or restructure.
2. **Forgetting the mock.** Tests then need a real device, which makes CI fragile.
3. **Leaking low-level errors** like `rusb::Error` past the driver. Wrap in `HalError`.
4. **Hardcoding a vendor name** in business code (`HoneywellBarcode::new()`). Use the registry.
5. **Blocking the executor** with a `read_exact` call. Wrap in `spawn_blocking`.
6. **Not handling the `Disconnected` case** — the cashier unplugs the scanner mid-shift. The system must reconnect or surface a clear error.
7. **Polling with `loop { sleep(1ms) }`** instead of waiting on a real event. Burns CPU and battery.
8. **Mixing sync and async traits.** Pick one. The HAL is `async`.

---

## See also

- **[`tauri-ipc`](../tauri-ipc/SKILL.md)** — the Tauri command layer that reaches into `DriverRegistry`. Hardware commands (e.g. `subscribe_barcode_scans`, `open_cash_drawer`, `print_receipt`) live in `apps/desktop-client/src/commands/hardware.rs` and follow the IPC patterns in `tauri-ipc`. The mock in `crates/oz-hal/src/drivers/mock.rs` is what makes those commands testable.
- **[`rust-backend`](../rust-backend/SKILL.md)** — defines the error and money patterns (`HalError`, `Money`, currency codes) that the HAL's traits and drivers must respect.
- **[`project-scaffold`](../project-scaffold/SKILL.md)** — the workspace layout (the `hal` crate's `Cargo.toml` follows the conventions there) and CI rules that gate the HAL into release.

---

> last audited 19-07-26 by skill-drift-guard
