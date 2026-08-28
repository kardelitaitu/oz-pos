//! Thumbnail generation.
//!
//! Decodes an image with the `image` crate, downscales it with a
//! high-quality filter while preserving aspect ratio, and re-encodes it.
//! The output is always an RGB(A) image, so a paletted/CMYK source is
//! normalised to a web-safe encoding.

use std::io::Cursor;

use image::imageops::FilterType;

use crate::{ImageDimensions, MediaError};

/// Generate a thumbnail of `image_data` fitting within `max_dimensions`.
///
/// # Arguments
///
/// * `image_data` — raw image bytes (JPEG / PNG / WebP / GIF / BMP).
/// * `max_dimensions` — the maximum bounding box for the thumbnail.
///   Aspect ratio is preserved; one side will match the bound exactly,
///   the other will be `<=` the bound.
///
/// # Returns
///
/// The re-encoded thumbnail bytes (JPEG) and its resulting dimensions.
///
/// # Errors
///
/// [`MediaError::InvalidImage`] if the input cannot be decoded, or
/// [`MediaError::InvalidDimensions`] if the source is degenerate
/// (zero width or height).
pub fn generate_thumbnail(
    image_data: &[u8],
    max_dimensions: ImageDimensions,
) -> Result<(Vec<u8>, ImageDimensions), MediaError> {
    if max_dimensions.width == 0 || max_dimensions.height == 0 {
        return Err(MediaError::InvalidDimensions(
            "max dimensions must be positive".into(),
        ));
    }

    let img = image::load_from_memory(image_data)
        .map_err(|e| MediaError::InvalidImage(format!("decode: {e}")))?;

    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(MediaError::InvalidDimensions(
            "source image has zero size".into(),
        ));
    }

    // Compute the largest dimensions that fit the box while preserving
    // aspect ratio. Use u64 math to avoid overflow on huge sources.
    let scale =
        (max_dimensions.width as u64).min(max_dimensions.height as u64 * w as u64 / h as u64);
    let new_w = scale.max(1) as u32;
    let new_h = (new_w as u64 * h as u64 / w as u64).max(1) as u32;

    // Triangle (bilinear) is the recommended high-quality downscale
    // filter; CatmullRom is sharper but slower. Triangle is the standard
    // trade-off for thumbnails.
    let thumb = img.resize(new_w, new_h, FilterType::Triangle);
    let rgb = thumb.to_rgb8();

    let mut out = Vec::new();
    rgb.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
        .map_err(|e| MediaError::InvalidImage(format!("encode: {e}")))?;

    Ok((out, ImageDimensions::new(new_w, new_h)))
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
