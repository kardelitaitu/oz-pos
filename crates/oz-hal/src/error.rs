//! Error type for the Hardware Abstraction Layer.
//!
//! Every trait method in `oz-hal` returns `Result<T, HalError>`. The enum
//! is `#[non_exhaustive]` so new failure modes can be added without
//! breaking semver. Drivers convert third-party errors with
//! `.map_err(|e| HalError::Usb(e.to_string()))` at the trait boundary
//! — never leak `rusb`/`btleplug`/`serialport` types past the driver.

use serde::Serialize;
use thiserror::Error;

/// Serializable discriminator for [`HalError`] variants.
///
/// Mirrored on the front-end as `AppError.subKind` so UI code can branch
/// on the specific hardware failure mode without parsing the message string.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HalErrorKind {
    /// Device id not found in the registry.
    NotFound,
    /// Device was disconnected.
    Disconnected,
    /// I/O transport error.
    Io,
    /// USB transport error.
    Usb,
    /// Bluetooth transport error.
    Bluetooth,
    /// Operation timed out.
    Timeout,
    /// Malformed packet or unexpected response.
    Protocol,
    /// Device is busy with a prior request.
    Busy,
}

/// Errors that can originate in a HAL driver or the HAL runtime.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HalError {
    /// The requested device id was not in the registry.
    #[error("device not found: {0}")]
    NotFound(String),

    /// The device was connected but the user/environment disconnected it.
    #[error("device disconnected")]
    Disconnected,

    /// An `std::io::Error` bubbled up from the transport layer.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// A USB transport error.
    #[error("usb error: {0}")]
    Usb(String),

    /// A Bluetooth transport error.
    #[error("bluetooth error: {0}")]
    Bluetooth(String),

    /// An operation did not complete within its timeout window.
    #[error("operation timed out after {0} ms")]
    Timeout(u32),

    /// The device returned a malformed packet or unexpected response.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The device is busy with a previous request.
    #[error("device busy")]
    Busy,
}

impl HalError {
    /// Map a `HalError` to its [`HalErrorKind`] discriminator.
    pub fn kind(&self) -> HalErrorKind {
        match self {
            HalError::NotFound(_) => HalErrorKind::NotFound,
            HalError::Disconnected => HalErrorKind::Disconnected,
            HalError::Io(_) => HalErrorKind::Io,
            HalError::Usb(_) => HalErrorKind::Usb,
            HalError::Bluetooth(_) => HalErrorKind::Bluetooth,
            HalError::Timeout(_) => HalErrorKind::Timeout,
            HalError::Protocol(_) => HalErrorKind::Protocol,
            HalError::Busy => HalErrorKind::Busy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_id() {
        let e = HalError::NotFound("scanner-01".into());
        assert_eq!(e.to_string(), "device not found: scanner-01");
    }

    #[test]
    fn timeout_displays_ms() {
        let e = HalError::Timeout(250);
        assert_eq!(e.to_string(), "operation timed out after 250 ms");
    }

    #[test]
    fn io_conversion_via_from() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: HalError = io.into();
        assert!(matches!(e, HalError::Io(_)));
    }

    #[test]
    fn disconnected_display() {
        let e = HalError::Disconnected;
        assert_eq!(e.to_string(), "device disconnected");
    }

    #[test]
    fn usb_display() {
        let e = HalError::Usb("permission denied".into());
        assert_eq!(e.to_string(), "usb error: permission denied");
    }

    #[test]
    fn bluetooth_display() {
        let e = HalError::Bluetooth("adapter not found".into());
        assert_eq!(e.to_string(), "bluetooth error: adapter not found");
    }

    #[test]
    fn protocol_display() {
        let e = HalError::Protocol("unexpected NAK".into());
        assert_eq!(e.to_string(), "protocol error: unexpected NAK");
    }

    #[test]
    fn busy_display() {
        let e = HalError::Busy;
        assert_eq!(e.to_string(), "device busy");
    }
}
