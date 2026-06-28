//! TCP / network receipt printer driver.
//!
//! Implements [`ReceiptPrinter`] over raw TCP (port 9100). Many POS
//! printers support this directly: Epson TM-i series, Star mC-Print3,
//! Bixolon SRP-350plus, etc. The printer receives the data as-is and
//! interprets it as ESC/POS commands.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::error::HalError;
use crate::traits::printer::ReceiptPrinter;
use crate::transport::tcp;
use crate::types::DeviceInfo;

use super::escpos;

/// A receipt printer driven through a raw TCP (port 9100) connection.
pub struct TcpReceiptPrinter {
    addr: String,
    stream: Arc<Mutex<Option<TcpStream>>>,
    info: DeviceInfo,
    partial_cut: bool,
}

impl TcpReceiptPrinter {
    /// Create a new TCP printer targetting the given address.
    ///
    /// `addr` can be an IP (`"192.168.1.100"`) or hostname
    /// (`"printer.local"`). Port 9100 is used unless specified as
    /// `"host:port"`.
    pub fn new(addr: impl Into<String>, info: DeviceInfo) -> Self {
        Self {
            addr: addr.into(),
            stream: Arc::new(Mutex::new(None)),
            info,
            partial_cut: false,
        }
    }

    /// Set whether to use a partial cut instead of full cut.
    pub fn with_partial_cut(mut self, partial: bool) -> Self {
        self.partial_cut = partial;
        self
    }

    async fn ensure_connected(&self) -> Result<(), HalError> {
        let mut guard = self.stream.lock().await;
        if guard.is_some() {
            return Ok(());
        }

        let stream = tcp::connect(&self.addr).await?;
        *guard = Some(stream);
        Ok(())
    }

    async fn write_to_stream(&self, data: &[u8]) -> Result<(), HalError> {
        let mut guard = self.stream.lock().await;
        let stream = guard
            .as_mut()
            .ok_or(HalError::NotFound("not connected".into()))?;

        tcp::write_all(stream, data).await
    }
}

#[async_trait]
impl ReceiptPrinter for TcpReceiptPrinter {
    async fn print_receipt(&self, body: &str) -> Result<(), HalError> {
        self.ensure_connected().await?;
        let data = escpos::format_receipt(body);
        self.write_to_stream(&data).await
    }

    async fn cut(&self) -> Result<(), HalError> {
        self.ensure_connected().await?;
        let data = if self.partial_cut {
            escpos::CUT_PARTIAL.to_vec()
        } else {
            escpos::CUT_FULL.to_vec()
        };
        self.write_to_stream(&data).await
    }

    fn device_info(&self) -> DeviceInfo {
        self.info.clone()
    }
}
