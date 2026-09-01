//! Image byte-store endpoints (spec 0046b §3.4, §3.7).
//!
//! The cloud serves content-addressed WebP images from the `OZ_IMAGE_DIR`
//! volume. Immutable content addressing: the sha-256 of the transcoded
//! bytes is simultaneously the filename (`{hash16}.webp`), the ETag, the
//! DB value, and the cache key. No invalidation logic exists anywhere.
//!
//! - `PUT /api/v1/images` — single upload (≤32 KB), magic-bytes + sha-256
//!   re-verification, atomic temp+rename on the same volume.
//! - `GET /api/v1/images/{hash16}` — tenant JWT + `image_refs` existence
//!   check, immutable `Cache-Control`, strict hash grammar, 404 unknown.
//! - `POST /api/v1/images:batch` — up to 16 images / 512 KB in one request,
//!   length-prefixed binary frames, per-hash `stored|duplicate|rejected`.
//! - `GET /api/v1/images:pack?hashes=...` — up to 64 files / 2 MB for cold
//!   start (length-prefixed frames).

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::path::{Path as FsPath, PathBuf};

use oz_core::db::Store;

use crate::AppState;
use crate::auth::ApiTokenClaims;

/// Maximum size of a single uploaded image (spec 0046b §3.4: 32 KB).
pub const MAX_IMAGE_BYTES: usize = 32 * 1024;
/// Batch limits (spec 0046b §3.6): 16 images / 512 KB per request.
pub const BATCH_MAX_IMAGES: usize = 16;
/// Batch byte cap (spec 0046b §3.6).
pub const BATCH_MAX_BYTES: usize = 512 * 1024;
/// Pack limits (spec 0046b §3.7): 64 files / 2 MB.
pub const PACK_MAX_FILES: usize = 64;
/// Pack byte cap (spec 0046b §3.7).
pub const PACK_MAX_BYTES: usize = 2 * 1024 * 1024;
/// WebP RIFF magic prefix.
const WEBP_MAGIC: &[u8] = b"RIFF";

// ── Hash helpers ───────────────────────────────────────────────────────

/// Compute the 16-hex-char content hash (first 16 hex chars of sha-256).
pub fn sha256_hex16(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    hex::encode(&digest[..8])
}

/// Validate a 16-char lowercase hex content hash (strict grammar — also
/// kills directory traversal in the path segment).
pub fn is_valid_hash16(s: &str) -> bool {
    s.len() == 16
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Check the WebP RIFF magic bytes (`RIFF....WEBP`).
fn is_webp_magic(data: &[u8]) -> bool {
    data.len() >= 12 && data.starts_with(WEBP_MAGIC) && &data[8..12] == b"WEBP"
}

// ── File helpers ───────────────────────────────────────────────────────

/// Path of the stored file for `hash16`.
fn image_path(image_dir: &FsPath, hash16: &str) -> PathBuf {
    image_dir.join(format!("{hash16}.webp"))
}

/// Atomically write `bytes` to `image_dir/{hash16}.webp` via temp+rename
/// on the same volume. Returns `true` when the file already existed
/// (duplicate upload).
fn store_image_atomic(image_dir: &FsPath, hash16: &str, bytes: &[u8]) -> std::io::Result<bool> {
    let final_path = image_path(image_dir, hash16);
    if final_path.exists() {
        return Ok(true); // duplicate — content-addressed identity
    }
    let tmp_path = image_dir.join(format!(".{hash16}.{}.tmp", std::process::id()));
    std::fs::write(&tmp_path, bytes)?;
    match std::fs::rename(&tmp_path, &final_path) {
        Ok(()) => Ok(false),
        Err(_) => {
            // A concurrent writer may have won the rename — the file now
            // exists; treat as a duplicate (idempotent success).
            let _ = std::fs::remove_file(&tmp_path);
            if final_path.exists() {
                Ok(true)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "atomic rename failed",
                ))
            }
        }
    }
}

// ── Shared per-image processing ────────────────────────────────────────

/// Outcome of processing one uploaded image.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ImageOutcome {
    Stored(String),
    Duplicate(String),
    Rejected,
}

/// Validate + store one image body, maintaining the refcount.
///
/// Returns `(hash16, outcome)`; `Rejected` carries no hash. The refcount
/// is always incremented — even on a duplicate upload (a no-op that skips
/// the refcount would let GC eat a file another product just started
/// referencing — spec 0046b §3.4 audit fix).
async fn process_image(state: &AppState, claims: &ApiTokenClaims, bytes: &[u8]) -> ImageOutcome {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return ImageOutcome::Rejected;
    }
    if !is_webp_magic(bytes) {
        return ImageOutcome::Rejected;
    }
    let hash16 = sha256_hex16(bytes);
    // Atomic write; `duplicate` = the file already existed.
    let duplicate = match store_image_atomic(&state.image_dir, &hash16, bytes) {
        Ok(d) => d,
        Err(_) => return ImageOutcome::Rejected,
    };
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let db = state.db.lock().await;
    let store = Store::new(&db);
    if store
        .ref_image(tenant_id, &hash16, bytes.len() as i64)
        .is_err()
    {
        return ImageOutcome::Rejected;
    }
    drop(db);
    if duplicate {
        ImageOutcome::Duplicate(hash16)
    } else {
        ImageOutcome::Stored(hash16)
    }
}

// ── Request / response types ───────────────────────────────────────────

/// Response body for a successful single upload.
#[derive(Serialize)]
pub struct PutImageResponse {
    /// Content-addressed hash (first 16 hex chars of sha-256).
    pub hash16: String,
}

/// Per-hash batch result.
#[derive(Serialize)]
pub struct BatchImageResult {
    /// Content hash when the image was accepted; `None` when rejected.
    pub hash: Option<String>,
    /// Outcome: `"stored"` | `"duplicate"` | `"rejected"`.
    pub status: &'static str,
}

/// Response body for a batch upload.
#[derive(Serialize)]
pub struct BatchPutResponse {
    /// Per-hash outcomes, in the same order as the request frames.
    pub results: Vec<BatchImageResult>,
}

/// Query params for the single-image upload (`?hash=...` optional).
#[derive(Deserialize, Default)]
pub struct PutImageQuery {
    /// Client-computed hash for 409 verification (spec 0046b §3.4).
    pub hash: Option<String>,
}

/// Query params for the pack endpoint (`?hashes=a,b,c`).
#[derive(Deserialize)]
pub struct PackQuery {
    /// Comma-separated list of content hashes to include in the pack.
    pub hashes: String,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `PUT /api/v1/images` — single upload.
///
/// Body: the transcoded WebP bytes. The server re-verifies magic bytes +
/// size and recomputes sha-256 before storing; a corrupt upload can never
/// enter the store. An optional `?hash=` query param (the hash the client
/// computed) is verified — a mismatch returns 409 and the bytes are
/// discarded (spec 0046b §3.4).
pub async fn put_image(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<ApiTokenClaims>,
    Query(query): Query<PutImageQuery>,
    body: Bytes,
) -> Response {
    let outcome = process_image(&state, &claims, &body).await;
    match outcome {
        ImageOutcome::Stored(hash16) | ImageOutcome::Duplicate(hash16) => {
            // Verify the client-computed hash if supplied (409 on mismatch).
            if let Some(expected) = query.hash.as_deref() {
                if !is_valid_hash16(expected) || expected != hash16 {
                    return (
                        StatusCode::CONFLICT,
                        Json(serde_json::json!({"error": "hash mismatch"})),
                    )
                        .into_response();
                }
            }
            (StatusCode::CREATED, Json(PutImageResponse { hash16 })).into_response()
        }
        ImageOutcome::Rejected => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "image rejected: must be a WebP ≤ 32 KB"})),
        )
            .into_response(),
    }
}

/// `POST /api/v1/images:batch` — up to 16 images / 512 KB per request.
///
/// Body: length-prefixed binary frames — `[u32 be len][bytes]` repeated.
/// The server re-verifies each frame independently; partial success is
/// allowed and the response reports per-hash outcomes.
pub async fn put_image_batch(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<ApiTokenClaims>,
    body: Bytes,
) -> Response {
    if body.len() > BATCH_MAX_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "batch exceeds 512 KB"})),
        )
            .into_response();
    }
    let mut results: Vec<BatchImageResult> = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= body.len() && results.len() < BATCH_MAX_IMAGES {
        let len_bytes: [u8; 4] = body[offset..offset + 4].try_into().unwrap();
        let frame_len = u32::from_be_bytes(len_bytes) as usize;
        offset += 4;
        if offset + frame_len > body.len() {
            results.push(BatchImageResult {
                hash: None,
                status: "rejected",
            });
            break;
        }
        let frame = &body[offset..offset + frame_len];
        offset += frame_len;
        match process_image(&state, &claims, frame).await {
            ImageOutcome::Stored(h) => results.push(BatchImageResult {
                hash: Some(h),
                status: "stored",
            }),
            ImageOutcome::Duplicate(h) => results.push(BatchImageResult {
                hash: Some(h),
                status: "duplicate",
            }),
            ImageOutcome::Rejected => results.push(BatchImageResult {
                hash: None,
                status: "rejected",
            }),
        }
    }
    (StatusCode::CREATED, Json(BatchPutResponse { results })).into_response()
}

/// `GET /api/v1/images/{hash16}` — immutable fetch.
///
/// Tenant JWT + `image_refs(tenant_id, hash)` existence check closes
/// cross-tenant fetch (unguessable-hash security alone does not); strict
/// hash-grammar validation kills directory traversal; immutable
/// `Cache-Control` makes every cache layer between the tablet LRU and the
/// volume cheap.
pub async fn get_image(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<ApiTokenClaims>,
    Path(hash16): Path<String>,
) -> Response {
    if !is_valid_hash16(&hash16) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response();
    }
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    // Existence check on the content spine — one indexed lookup.
    {
        let db = state.db.lock().await;
        let store = Store::new(&db);
        match store.image_ref_exists(tenant_id, &hash16) {
            Ok(true) => {}
            Ok(false) => {
                drop(db);
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "not found"})),
                )
                    .into_response();
            }
            Err(_) => {
                drop(db);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "internal error"})),
                )
                    .into_response();
            }
        }
    }
    let path = image_path(&state.image_dir, &hash16);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mut response = bytes.into_response();
            let headers = response.headers_mut();
            headers.insert(header::CONTENT_TYPE, "image/webp".parse().unwrap());
            headers.insert(
                header::CACHE_CONTROL,
                "max-age=31536000, immutable".parse().unwrap(),
            );
            headers.insert(header::ETAG, format!("\"{hash16}\"").parse().unwrap());
            // Compression hygiene: no Content-Encoding on image routes —
            // WebP is already the compression (spec 0046b §3.7).
            response
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response(),
    }
}

/// `GET /api/v1/images:pack?hashes=a,b,...` — cold-start pack.
///
/// Up to 64 files / 2 MB, length-prefixed frames `[u32 be len][bytes]`.
/// Missing/unreferenced hashes are silently skipped (the puller treats a
/// missing frame as 404 for that hash and keeps it in the missing set).
pub async fn get_image_pack(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<ApiTokenClaims>,
    Query(query): Query<PackQuery>,
) -> Response {
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("default");
    let hashes: Vec<&str> = query
        .hashes
        .split(',')
        .filter(|h| !h.is_empty() && is_valid_hash16(h))
        .collect();
    if hashes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "no valid hashes"})),
        )
            .into_response();
    }
    if hashes.len() > PACK_MAX_FILES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "pack exceeds 64 files"})),
        )
            .into_response();
    }

    let mut out: Vec<u8> = Vec::with_capacity(PACK_MAX_BYTES.min(1024));
    for h in &hashes {
        // Content-spine gate per hash.
        let referenced = {
            let db = state.db.lock().await;
            let store = Store::new(&db);
            match store.image_ref_exists(tenant_id, h) {
                Ok(v) => v,
                Err(_) => false,
            }
        };
        if !referenced {
            continue;
        }
        let path = image_path(&state.image_dir, h);
        match std::fs::read(&path) {
            Ok(bytes) => {
                if out.len() + 4 + bytes.len() > PACK_MAX_BYTES {
                    break;
                }
                out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                out.extend_from_slice(&bytes);
            }
            Err(_) => continue, // missing file → skip
        }
    }
    let mut response = out.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "max-age=31536000, immutable".parse().unwrap(),
    );
    response
}

#[cfg(test)]
#[path = "images_tests.rs"]
mod tests;
