//! Auto crop — STUB test placeholder.
//!
//! Tests will be added when the real implementation lands.

use crate::MediaError;
use crate::crop::{CropMode, auto_crop};

#[test]
fn stub_returns_not_implemented() {
    let result = auto_crop(b"fake-image-data", CropMode::TrimBorders, None);
    assert!(
        matches!(result, Err(MediaError::NotImplemented(_))),
        "expected NotImplemented error, got {result:?}"
    );
}
