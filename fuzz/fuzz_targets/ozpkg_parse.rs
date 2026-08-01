//! Fuzz target for `.ozpkg` archive parsing (PLG-01/PLG-06 boundary).
//!
//! Feeds arbitrary bytes to `OzpkArchive::from_bytes` — the same entry
//! classification, manifest validation, and resource-limit enforcement that
//! a future package installer would run over an untrusted archive. The
//! parser must:
//!
//!   * never panic on any byte sequence (traversal names, absurd sizes,
//!     compression-ratio bombs, malformed zip headers),
//!   * reject path-traversal entries, and
//!   * enforce the archive resource limits (entry count, compressed /
//!     uncompressed sizes, compression ratio).
//!
//! A panic here is a real bug in the hardened parser — the fuzz gate in
//! CI uploads the crashing input as an artifact.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // `from_bytes` runs the full parse: zip open, entry classification,
    // resource-limit checks, and manifest.json validation. Errors are the
    // expected outcome for most inputs — the invariant is that any input
    // either parses cleanly or fails with an error, never a panic.
    let _ = oz_plugin::package::OzpkArchive::from_bytes(data, "fuzz.ozpkg");
});
