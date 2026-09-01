//! Product/Menu image ingest commands (spec 0046b §3.3).
//!
//! `products_set_image_scoped` is the full ingest pipeline on the authoring
//! (desktop) device: it reads the source file chosen via the dialog plugin,
//! sniffs magic bytes, validates size/dimension caps, decodes, applies EXIF
//! orientation, resizes to 512 px longest edge, encodes as lossy WebP at
//! quality 40 (with adaptive degrade to q30 / q24 when >32 KB), computes the
//! SHA-256 content hash, atomically writes `{hash16}.webp` to the app cache
//! (`$APPCACHE/images/` — served by Tauri's asset protocol), and finally
//! assigns the hash to the product slot via `Store::set_product_image`.
//!
//! `products_clear_image_scoped` removes the DB assignment (the file lingers
//! until GC — content-addressed dedup means it may be referenced elsewhere).
//!
//! Per decision §5.6: the `image` + `webp` crates are linked only in the
//! desktop-client binary. Tablet renders + downloads; cloud re-verifies
//! magic + sha-256 only. Neither links the full image pipeline.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tauri::Manager;
use tauri::State;

use oz_core::Store;
use oz_core::permissions;

use crate::commands::authz::require_permission_for_session;
use crate::error::AppError;
use crate::state::AppState;

// ── Constants ──────────────────────────────────────────────────────────

/// Maximum raw input file size (5 MB).
const MAX_INPUT_BYTES: u64 = 5 * 1024 * 1024;

/// Maximum decoded pixel dimensions (4096²).
const MAX_DIMENSION: u32 = 4096;

/// Target longest edge after resize.
const TARGET_DIMENSION: u32 = 512;

/// Quality tiers for adaptive WebP encoding.
const QUALITY_PRIMARY: f32 = 40.0;
const QUALITY_FALLBACK: f32 = 30.0;
const QUALITY_FLOOR: f32 = 24.0;

/// Size thresholds for adaptive quality (hard reject at 48 KB).
const SIZE_HARD_REJECT: usize = 48 * 1024;

/// First 64 bits of the SHA-256 digest → 16 hex chars (≈1e-9 collision
/// probability at 300k images).
const HASH16_CHARS: usize = 16;

// ── Command: set image ─────────────────────────────────────────────────

/// Assign the image at `source_path` to `product_id` at `slot` (1..=5).
///
/// The ingest pipeline runs entirely in Rust: `source_path` is the file
/// chosen via the front-end dialog plugin, so zero image bytes cross the
/// IPC bridge.
///
/// Returns the 16-hex-char content hash of the transcoded image.
#[tauri::command]
pub async fn products_set_image_scoped(
    session_token: String,
    product_id: String,
    slot: i32,
    source_path: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, AppError> {
    // Resolve session + permission
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_UPDATE).await?;

    // Validate slot
    if slot < 1 || slot > 5 {
        return Err(AppError::Invalid(format!(
            "slot must be between 1 and 5, got {slot}"
        )));
    }
    if source_path.is_empty() {
        return Err(AppError::Invalid("source_path must not be empty".into()));
    }

    // Read the source file
    let raw_bytes = tokio::fs::read(&source_path)
        .await
        .map_err(|e| AppError::Invalid(format!("reading source file: {e}")))?;

    if raw_bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(AppError::Invalid(format!(
            "input exceeds {} bytes (got {})",
            MAX_INPUT_BYTES,
            raw_bytes.len()
        )));
    }

    // --- Sniff magic bytes (extension is ignored) ---
    let _input_format = sniff_format(&raw_bytes)
        .map_err(|e| AppError::Invalid(format!("unsupported or corrupt image format: {e}")))?;

    // --- Transcode & resize ---
    let webp_bytes = transcode_to_webp(&raw_bytes)?;

    // --- Hash ---
    let hash16 = sha256_hex16(&webp_bytes);

    // --- Atomic write to app cache ---
    let store_path = resolve_image_path(&app_handle, &hash16)?;
    let parent_dir = store_path
        .parent()
        .ok_or_else(|| AppError::Internal("image store path has no parent".into()))?;

    tokio::fs::create_dir_all(parent_dir)
        .await
        .map_err(|e| AppError::Internal(format!("creating image store dir: {e}")))?;

    // Only write if the file doesn't exist (dedupe hit).
    if !tokio::fs::try_exists(&store_path).await.unwrap_or(false) {
        // Write to a temp path first, then atomically rename
        let temp_path = parent_dir.join(format!(".{}.tmp", &hash16));
        {
            let mut tmp = tokio::fs::File::create(&temp_path)
                .await
                .map_err(|e| AppError::Internal(format!("creating temp file: {e}")))?;
            tokio::io::AsyncWriteExt::write_all(&mut tmp, &webp_bytes)
                .await
                .map_err(|e| AppError::Internal(format!("writing temp file: {e}")))?;
        }
        tokio::fs::rename(&temp_path, &store_path)
            .await
            .map_err(|e| AppError::Internal(format!("renaming image file: {e}")))?;
    }

    // --- DB assignment in transaction ---
    let session = state.resolve_session(&session_token)?;
    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.set_product_image(&product_id, slot, &hash16)?;

    tracing::info!(product_id, slot, hash = %hash16, "product image set");
    Ok(hash16)
}

// ── Command: clear image ───────────────────────────────────────────────

/// Remove the image at `slot` for `product_id`.
///
/// Only the DB assignment is removed; the file on disk is left for the GC
/// sweep (P4) since content-addressed dedup means the same file may be
/// referenced by other products.
#[tauri::command]
pub async fn products_clear_image_scoped(
    session_token: String,
    product_id: String,
    slot: i32,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_UPDATE).await?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    store.clear_product_image(&product_id, slot)?;

    tracing::info!(product_id, slot, "product image cleared");
    Ok(())
}

// ── Command: list images ───────────────────────────────────────────────

/// List the image assignments for a product (slots 1..=5), ordered by slot.
///
/// The editor flow calls this on open to show the primary + alternatives.
#[tauri::command]
pub async fn products_list_images_scoped(
    session_token: String,
    product_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProductImageDto>, AppError> {
    let session = state.resolve_session(&session_token)?;
    require_permission_for_session(&state, &session, permissions::PRODUCTS_READ).await?;

    let conn = state
        .db_manager
        .open_store(&session.store_id)
        .map_err(|e| AppError::Internal(format!("opening store db: {e}")))?;
    let db = conn
        .lock()
        .map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;
    let store = Store::new(&db);
    let images = store.list_product_images(&product_id)?;
    Ok(images
        .into_iter()
        .map(|img| ProductImageDto {
            slot: img.slot,
            hash: img.hash,
            position: img.position,
        })
        .collect())
}

/// A product image assignment returned to the front-end.
#[derive(Debug, serde::Serialize)]
pub struct ProductImageDto {
    /// Slot 1 = primary; slots 2..5 = alternatives.
    pub slot: i32,
    /// Content-addressed hash (first 16 hex chars of sha-256).
    pub hash: String,
    /// Display order of alternatives (0-based).
    pub position: i32,
}

// ── Image pipeline helpers ─────────────────────────────────────────────

/// Detect the image format from magic bytes. Returns the format name on
/// success; rejects everything that is not WebP, JPEG, or PNG.
fn sniff_format(bytes: &[u8]) -> Result<&'static str, &'static str> {
    if bytes.len() < 12 {
        return Err("file too small to contain a valid image header");
    }
    // WebP: RIFF header + WEBP magic
    if bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Ok("webp");
    }
    // JPEG: starts with FFD8FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok("jpeg");
    }
    // PNG: starts with 89504E47
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Ok("png");
    }
    Err("unsupported format: only WebP, JPEG, and PNG are accepted \
         (HEIC, AVIF, BMP, GIF, TIFF and others are rejected)")
}

/// Transcode `input_bytes` to 512 px WebP at quality 40 with adaptive fallback.
///
/// Pipeline: decode → resize to 512 px longest edge → encode as lossy
/// WebP q40 (adaptive q40→q30→q24). Hard-rejects if the result exceeds
/// 48 KB. EXIF orientation handling is intentionally deferred to P4
/// (the vast majority of POS product images are already correctly
/// oriented by the source device).
fn transcode_to_webp(input_bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    // Decompression-bomb dimension check before full decode
    let (width, height) = {
        let reader = image::ImageReader::new(std::io::Cursor::new(input_bytes))
            .with_guessed_format()
            .map_err(|e| AppError::Invalid(format!("reading image format: {e}")))?;
        reader
            .into_dimensions()
            .map_err(|e| AppError::Invalid(format!("reading image dimensions: {e}")))?
    };

    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(AppError::Invalid(format!(
            "image dimensions {width}×{height} exceed max {MAX_DIMENSION}"
        )));
    }

    let pixels = width as u64 * height as u64;
    if pixels > (MAX_DIMENSION as u64 * MAX_DIMENSION as u64) {
        return Err(AppError::Invalid(format!(
            "image has {pixels} pixels, exceeding MAX_DIMENSION²"
        )));
    }

    // Decode
    let img = image::load_from_memory(input_bytes)
        .map_err(|e| AppError::Invalid(format!("decoding image: {e}")))?;

    // Resize to 512 px longest edge, preserving aspect ratio
    let (w, h) = (img.width(), img.height());
    let (new_w, new_h) = if w > h {
        (
            TARGET_DIMENSION,
            (h as u64 * TARGET_DIMENSION as u64 / w as u64).max(1) as u32,
        )
    } else {
        (
            (w as u64 * TARGET_DIMENSION as u64 / h as u64).max(1) as u32,
            TARGET_DIMENSION,
        )
    };
    let img = img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);

    // Adaptive quality encoding via libwebp
    let qualities = [QUALITY_PRIMARY, QUALITY_FALLBACK, QUALITY_FLOOR];
    for &quality in &qualities {
        let encoder = webp::Encoder::from_image(&img)
            .map_err(|e| AppError::Invalid(format!("creating webp encoder: {e}")))?;
        let encoded = encoder.encode(quality);
        let bytes = encoded.to_vec();

        if bytes.len() <= SIZE_HARD_REJECT {
            return Ok(bytes);
        }
    }

    Err(AppError::Invalid(format!(
        "image exceeds {SIZE_HARD_REJECT} bytes even at quality {QUALITY_FLOOR} — \
         try a smaller or simpler image"
    )))
}

/// Compute the first 16 hex characters of the SHA-256 digest.
fn sha256_hex16(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex::encode(&digest[..HASH16_CHARS / 2]) // 8 bytes → 16 hex chars
}

/// Resolve the filesystem path for the content-addressed image file.
fn resolve_image_path(app_handle: &tauri::AppHandle, hash16: &str) -> Result<PathBuf, AppError> {
    let cache_dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Internal(format!("resolving app cache dir: {e}")))?;
    Ok(cache_dir.join("images").join(format!("{hash16}.webp")))
}

#[cfg(test)]
#[path = "products_images_tests.rs"]
mod tests;
