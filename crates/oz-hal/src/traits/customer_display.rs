//! `CustomerDisplay` — a secondary screen that shows the cart total
//! and line count to the customer.
//!
//! Typical hardware is a 2-line × 20-character or 2-line × 16-character
//! LCD/VFD pole display connected over serial (RS-232 or USB-to-serial).
//! The display is updated in real time as items are scanned.

use async_trait::async_trait;

use crate::error::HalError;
use crate::types::DeviceInfo;

/// Data pushed to the customer display whenever the cart changes.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayContent {
    /// First line — typically the store name or a greeting.
    pub line1: String,
    /// Second line — typically the current total.
    pub line2: String,
}

/// A secondary display that shows cart information to the customer.
///
/// Drivers communicate over serial/USB/HID using the display's native
/// protocol. The trait provides high-level methods for the common
/// operations: clear, show content, set brightness.
#[async_trait]
pub trait CustomerDisplay: Send + Sync {
    /// Connect to the display device.
    async fn connect(&self) -> Result<Box<dyn CustomerDisplay>, HalError>;

    /// Show the given content on the display (two lines).
    /// Implementations should clear the screen first, then write
    /// the two lines at the appropriate positions.
    async fn show(&self, content: &DisplayContent) -> Result<(), HalError>;

    /// Clear the display entirely.
    async fn clear(&self) -> Result<(), HalError>;

    /// Set display brightness (0.0 = off, 1.0 = max).
    /// Returns an error if the display doesn't support brightness control.
    async fn set_brightness(&self, level: f32) -> Result<(), HalError>;

    /// Device identity, used in logs and the setup wizard.
    fn device_info(&self) -> DeviceInfo;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::mock::MockCustomerDisplay;

    #[tokio::test]
    async fn mock_defaults() {
        let d = MockCustomerDisplay::new();
        let info = d.device_info();
        assert_eq!(info.vendor, "mock");
    }

    #[tokio::test]
    async fn show_and_clear() {
        let d = MockCustomerDisplay::new();
        let content = DisplayContent {
            line1: "OZ MART".into(),
            line2: "Total: $12.50".into(),
        };
        d.show(&content).await.unwrap();
        assert_eq!(d.last_content(), Some(content));

        d.clear().await.unwrap();
        assert_eq!(d.last_content(), None);
    }

    #[tokio::test]
    async fn brightness_defaults_to_max() {
        let d = MockCustomerDisplay::new();
        assert!((d.brightness() - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn set_brightness_clamps() {
        let d = MockCustomerDisplay::new();
        d.set_brightness(0.5).await.unwrap();
        assert!((d.brightness() - 0.5).abs() < f32::EPSILON);

        d.set_brightness(1.5).await.unwrap();
        assert!((d.brightness() - 1.0).abs() < f32::EPSILON);

        d.set_brightness(-0.1).await.unwrap();
        assert!((d.brightness() - 0.0).abs() < f32::EPSILON);
    }

    // ── DisplayContent struct tests ───────────────────────────────────

    #[test]
    fn display_content_debug() {
        let content = DisplayContent {
            line1: "OZ MART".into(),
            line2: "Total: $12.50".into(),
        };
        let debug = format!("{content:?}");
        assert!(debug.contains("OZ MART"));
        assert!(debug.contains("Total: $12.50"));
    }

    #[test]
    fn display_content_clone() {
        let content = DisplayContent {
            line1: "Line 1".into(),
            line2: "Line 2".into(),
        };
        let cloned = content.clone();
        assert_eq!(content, cloned);
        assert_eq!(cloned.line1, "Line 1");
        assert_eq!(cloned.line2, "Line 2");
    }

    #[test]
    fn display_content_partial_eq() {
        let a = DisplayContent {
            line1: "Hello".into(),
            line2: "World".into(),
        };
        let b = DisplayContent {
            line1: "Hello".into(),
            line2: "World".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn display_content_not_eq_when_lines_differ() {
        let a = DisplayContent {
            line1: "Hello".into(),
            line2: "World".into(),
        };
        let b = DisplayContent {
            line1: "Hello".into(),
            line2: "Different".into(),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn display_content_empty_strings() {
        let content = DisplayContent {
            line1: String::new(),
            line2: String::new(),
        };
        assert_eq!(content.line1, "");
        assert_eq!(content.line2, "");
    }

    #[test]
    fn display_content_field_values() {
        let content = DisplayContent {
            line1: "Store Name".into(),
            line2: "Rp 50,000".into(),
        };
        assert_eq!(content.line1, "Store Name");
        assert_eq!(content.line2, "Rp 50,000");
    }
}
