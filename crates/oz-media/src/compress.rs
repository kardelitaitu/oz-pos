//! Image compression — PLANNED (stub).
//!
//! Re-encodes an image at a target quality to reduce file size. The real
//! implementation will re-encode JPEG at a configurable quality, and
//! convert between formats (e.g. PNG → WebP) when smaller.

use crate::{ImageFormat, MediaError};

/// Compression quality setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quality {
    /// High quality, larger files (e.g. JPEG quality 90).
    High,
    /// Balanced quality/size (e.g. JPEG quality 80).
    Medium,
    /// Small files, acceptable quality (e.g. JPEG quality 65).
    Low,
}

/// Compress `image_data` to `target_format` at the given quality.
///
/// # STUB
///
/// Always returns [`MediaError::NotImplemented`] until implemented.
pub fn compress(
    image_data: &[u8],
    target_format: ImageFormat,
    quality: Quality,
) -> Result<Vec<u8>, MediaError> {
    let _ = (image_data, target_format, quality);
    Err(MediaError::NotImplemented(
        "image compression — PLANNED, not implemented yet".into(),
    ))
}

/// Estimate the compression ratio (output/input size) a caller can
/// expect. Returns `1.0` until real statistics are available.
pub fn expected_ratio(target_format: ImageFormat, quality: Quality) -> f64 {
    let _ = (target_format, quality);
    // PLANNED: replace with measured averages once implemented.
    1.0
}

#[cfg(test)]
#[path = "compress_tests.rs"]
mod tests;
