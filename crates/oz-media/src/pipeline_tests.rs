//! Media pipeline — STUB test placeholder + functional hash test.
//!
//! The pipeline is a stub until the real stages land; the content-hash
//! helper is functional and covered here.

use crate::pipeline::content_hash;

#[test]
fn content_hash_is_stable_sha256() {
    let h1 = content_hash(b"hello world");
    let h2 = content_hash(b"hello world");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
    assert_eq!(
        h1,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn content_hash_differs_for_diff_inputs() {
    assert_ne!(content_hash(b"a"), content_hash(b"b"));
}
