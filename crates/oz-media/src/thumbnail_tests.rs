//! Thumbnail — STUB test placeholder.
//!
//! Tests will be added when the real implementation lands.

use crate::thumbnail::generate_thumbnail;
use crate::{ImageDimensions, MediaError};

#[test]
fn stub_returns_not_implemented() {
    let result = generate_thumbnail(b"fake-image-data", ImageDimensions::new(128, 128));
    assert!(
        matches!(result, Err(MediaError::NotImplemented(_))),
        "expected NotImplemented error, got {result:?}"
    );
}
