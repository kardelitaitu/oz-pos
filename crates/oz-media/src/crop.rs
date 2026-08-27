//! Auto image crop — PLANNED (stub).
//!
//! Content-aware cropping for product photos. The real implementation
//! will trim uniform borders / letterboxing, or centre-crop an image to
//! a target aspect ratio while keeping the subject in frame.

use crate::{ImageDimensions, MediaError};

/// Crop behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CropMode {
    /// Trim uniform (background-coloured) borders automatically.
    TrimBorders,
    /// Centre-crop to the requested aspect ratio.
    CenterCrop,
    /// Smart-crop that keeps the detected subject in frame.
    Smart,
}

/// Auto-crop `image_data` according to `mode`.
///
/// # STUB
///
/// Always returns [`MediaError::NotImplemented`] until implemented.
pub fn auto_crop(
    image_data: &[u8],
    mode: CropMode,
    target: Option<ImageDimensions>,
) -> Result<(Vec<u8>, ImageDimensions), MediaError> {
    let _ = (image_data, mode, target);
    Err(MediaError::NotImplemented(
        "auto crop — PLANNED, not implemented yet".into(),
    ))
}

#[cfg(test)]
#[path = "crop_tests.rs"]
mod tests;
