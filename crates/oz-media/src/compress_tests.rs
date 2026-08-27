//! Image compression — STUB test placeholder.
//!
//! Tests will be added when the real implementation lands.

use crate::compress::compress;
use crate::{ImageFormat, MediaError, compress::Quality};

#[test]
fn stub_returns_not_implemented() {
    let result = compress(b"fake-image-data", ImageFormat::Jpeg, Quality::Medium);
    assert!(
        matches!(result, Err(MediaError::NotImplemented(_))),
        "expected NotImplemented error, got {result:?}"
    );
}
