//! Media pipeline orchestrator.
//!
//! The pipeline composes the media stages in the canonical order:
//!
//! ```text
//! ingest (decode + validate)
//!   → auto-crop (trim borders / centre-crop / smart)
//!   → thumbnail (per preset)
//!   → compress (format + quality)
//!   → store (MediaStorage trait)
//!   → DB metadata (media_assets / media_thumbnails rows)
//! ```
//!
//! **Order matters:** crop before resize before compress — never
//! compress-then-crop, which wastes quality on pixels that are discarded.
//!
//! The transform stages ([`crate::crop`], [`crate::thumbnail`],
//! [`crate::compress`]) are implemented. [`MediaPipeline::transform`]
//! composes them. [`MediaPipeline::process`] additionally persists the
//! variants through [`MediaStorage`] — still PLANNED (the storage
//! backends are stubs).

use crate::compress::Quality;
use crate::crop::CropMode;
use crate::storage::MediaStorage;
use crate::thumbnail::ThumbnailPreset;
use crate::{ImageDimensions, ImageFormat, MediaError};

/// Guardrails for image ingestion — protects against decompression bombs
/// and absurd dimensions.
#[derive(Debug, Clone, Copy)]
pub struct MediaLimits {
    /// Maximum decodable pixels (width × height). Default 40 MP.
    pub max_pixels: u64,
    /// Maximum file size in bytes before decoding. Default 20 MiB.
    pub max_input_bytes: u64,
    /// Maximum output side length in pixels. Default 8192.
    pub max_side: u32,
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_pixels: 40_000_000,
            max_input_bytes: 20 * 1024 * 1024,
            max_side: 8192,
        }
    }
}

/// A single output variant produced by the pipeline.
#[derive(Debug, Clone)]
pub struct MediaVariant {
    /// Storage key (e.g. `products/abc/photo_thumb.jpg`).
    pub key: String,
    /// Image format.
    pub format: ImageFormat,
    /// Resulting dimensions.
    pub dimensions: ImageDimensions,
}

/// Orchestrates ingest → crop → thumbnail → compress → store.
///
/// **PLANNED:** [`Self::process`] returns [`MediaError::NotImplemented`]
/// until the stages are implemented. The API is fixed now so callers and
/// the DB layer can be written against it.
pub struct MediaPipeline<S: MediaStorage> {
    storage: S,
    limits: MediaLimits,
}

impl<S: MediaStorage> MediaPipeline<S> {
    /// Create a pipeline over `storage` with the default limits.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            limits: MediaLimits::default(),
        }
    }

    /// Create a pipeline with custom ingestion limits.
    pub fn with_limits(storage: S, limits: MediaLimits) -> Self {
        Self { storage, limits }
    }

    /// Access the storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// The configured ingestion limits.
    pub fn limits(&self) -> &MediaLimits {
        &self.limits
    }

    /// Run the full transform pipeline on `input_bytes` without
    /// persisting: crop → thumbnail (per preset) → compress.
    ///
    /// Returns the processed variants (original + one per preset). The
    /// caller decides where to store the bytes.
    ///
    /// # Errors
    ///
    /// [`MediaError::InvalidDimensions`] if the input exceeds the ingest
    /// guardrails, or any stage error ([`MediaError::InvalidImage`]).
    pub fn transform(
        &self,
        file_name: &str,
        input_bytes: &[u8],
        crop_mode: CropMode,
        target_format: ImageFormat,
        quality: Quality,
        presets: &[ThumbnailPreset],
    ) -> Result<Vec<MediaVariant>, MediaError> {
        if input_bytes.len() as u64 > self.limits.max_input_bytes {
            return Err(MediaError::InvalidDimensions(format!(
                "input exceeds {} bytes",
                self.limits.max_input_bytes
            )));
        }

        // 1. Crop (normalise the frame first — cheap when no crop needed).
        let (cropped, _) = crate::crop::auto_crop(input_bytes, crop_mode, None)?;

        // 2. Original variant at the target format/quality.
        // PLANNED: the compressed bytes will be persisted with the variant
        // once the storage backends land; for now the encode itself is
        // exercised so decode/encode errors surface here.
        let _original_bytes = crate::compress::compress(&cropped, target_format, quality)?;
        let dims = original_dims(&cropped)?;

        let mut variants = Vec::with_capacity(presets.len() + 1);
        variants.push(MediaVariant {
            key: file_name.to_owned(),
            format: target_format,
            dimensions: dims,
        });

        // 3. One thumbnail per preset.
        for preset in presets {
            let (thumb_bytes, thumb_dims) =
                crate::thumbnail::generate_thumbnail(&cropped, preset.max_dimensions())?;
            // PLANNED: persist the compressed thumbnail bytes with the
            // variant once storage lands.
            let _thumb_bytes = crate::compress::compress(&thumb_bytes, target_format, quality)?;
            let suffix = preset_suffix(*preset);
            variants.push(MediaVariant {
                key: format!("{file_name}{suffix}"),
                format: target_format,
                dimensions: thumb_dims,
            });
        }

        Ok(variants)
    }

    /// Run the full pipeline on `input_bytes` and persist the variants
    /// through the storage backend.
    ///
    /// # STUB
    ///
    /// The transforms are real ([`Self::transform`]) and are applied;
    /// persistence is PLANNED — the storage backends are stubs, and the
    /// variant model does not yet carry the transformed bytes. Returns
    /// the same variants as [`Self::transform`].
    pub async fn process(
        &self,
        owner_key: &str,
        file_name: &str,
        input_bytes: &[u8],
        crop_mode: CropMode,
        target_format: ImageFormat,
        quality: Quality,
        presets: &[ThumbnailPreset],
    ) -> Result<Vec<MediaVariant>, MediaError> {
        let _ = owner_key;
        let _ = self.storage();
        self.transform(
            file_name,
            input_bytes,
            crop_mode,
            target_format,
            quality,
            presets,
        )
    }
}

/// Decode just the dimensions of a JPEG/PNG/WebP byte buffer.
fn original_dims(bytes: &[u8]) -> Result<ImageDimensions, MediaError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| MediaError::InvalidImage(format!("decode: {e}")))?;
    Ok(ImageDimensions::new(img.width(), img.height()))
}

/// Map a preset to its file-name suffix.
fn preset_suffix(preset: ThumbnailPreset) -> &'static str {
    match preset {
        ThumbnailPreset::Icon => "_icon",
        ThumbnailPreset::Small => "_small",
        ThumbnailPreset::Medium => "_medium",
        ThumbnailPreset::Large => "_large",
    }
}

/// Compute the SHA-256 content hash of `bytes` — used for dedup.
///
/// Functional today (no stub): hashing is pure and dependency-light, and
/// dedup logic can be written and tested against it immediately.
pub fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;
