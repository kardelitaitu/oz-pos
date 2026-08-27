//! Thumbnail generation — PLANNED (stub).
//!
//! Generates a downscaled version of an image while preserving the
//! aspect ratio. The real implementation will decode the input bytes
//! with the `image` crate, resize with a high-quality filter (e.g.
//! `image::imageops::FilterType::Triangle`), and re-encode.

use crate::{ImageDimensions, MediaError};

/// Generate a thumbnail of `image_data` fitting within `max_dimensions`.
///
/// # Arguments
///
/// * `image_data` — raw image bytes (JPEG / PNG / WebP).
/// * `max_dimensions` — the maximum bounding box for the thumbnail.
///   Aspect ratio is preserved; one side will match the bound exactly.
///
/// # Returns
///
/// The re-encoded thumbnail bytes and its resulting dimensions.
///
/// # STUB
///
/// Always returns [`MediaError::NotImplemented`] until implemented.
pub fn generate_thumbnail(
    image_data: &[u8],
    max_dimensions: ImageDimensions,
) -> Result<(Vec<u8>, ImageDimensions), MediaError> {
    let _ = (image_data, max_dimensions);
    Err(MediaError::NotImplemented(
        "thumbnail generation — PLANNED, not implemented yet".into(),
    ))
}

/// A named thumbnail size preset, so callers don't hard-code pixel
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThumbnailPreset {
    /// Small icon (e.g. 64×64).
    Icon,
    /// Small thumbnail (e.g. 128×128).
    Small,
    /// Medium thumbnail (e.g. 256×256).
    Medium,
    /// Large thumbnail (e.g. 512×512).
    Large,
}

impl ThumbnailPreset {
    /// The bounding-box dimensions for this preset.
    pub fn max_dimensions(&self) -> ImageDimensions {
        match self {
            Self::Icon => ImageDimensions::new(64, 64),
            Self::Small => ImageDimensions::new(128, 128),
            Self::Medium => ImageDimensions::new(256, 256),
            Self::Large => ImageDimensions::new(512, 512),
        }
    }
}

#[cfg(test)]
#[path = "thumbnail_tests.rs"]
mod tests;
