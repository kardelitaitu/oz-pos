//! `ReceiptPrinter` — the trait every receipt printer driver implements.

use async_trait::async_trait;

use crate::error::HalError;
use crate::types::DeviceInfo;

/// A device that prints customer receipts (and kitchen tickets, in the
/// future — that's a separate trait once it has more shape).
#[async_trait]
pub trait ReceiptPrinter: Send + Sync {
    /// Print a receipt. `body` is plain text; the driver is responsible
    /// for converting to the device's native format (ESC/POS, StarPRNT,
    /// etc.) and slicing the paper at the end.
    async fn print_receipt(&self, body: &str) -> Result<(), HalError>;

    /// Feed `n` blank lines after the receipt, then cut. Most drivers
    /// implement this as the standard ESC/POS sequence; a no-op default
    /// is provided for printers that don't expose a cutter.
    async fn cut(&self) -> Result<(), HalError> {
        Ok(())
    }

    /// Device identity, used in logs and the setup wizard.
    fn device_info(&self) -> DeviceInfo;
}
