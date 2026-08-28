//! Media pipeline orchestrator — PLANNED (stub).
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

    /// Run the full pipeline on `input_bytes`.
    ///
    /// Produces the original (stored) variant plus one thumbnail per
    /// requested preset. The original's `key` is derived from
    /// `owner_key` + `file_name`; thumbnails append a preset suffix.
    ///
    /// # STUB
    ///
    /// Always returns [`MediaError::NotImplemented`] until implemented.
    #[allow(clippy::too_many_arguments)] // planned stub — args will be grouped into a Config struct
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
        // Validate against the ingest guardrails now so the contract is
        // exercised even before the real decode lands.
        if input_bytes.len() as u64 > self.limits.max_input_bytes {
            return Err(MediaError::InvalidDimensions(format!(
                "input exceeds {} bytes",
                self.limits.max_input_bytes
            )));
        }

        let _ = (
            self.storage(),
            owner_key,
            file_name,
            crop_mode,
            target_format,
            quality,
            presets,
        );
        Err(MediaError::NotImplemented(
            "media pipeline process — PLANNED, not implemented yet".into(),
        ))
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
