//! Cryptographic helpers for encrypting sensitive data at rest.
//!
//! This module re-exports the standalone [`oz_crypto`] crate for backward
//! compatibility. All functions and types live in `oz-crypto` which can be
//! depended on by `platform-core` without creating a cyclic dependency.

// Re-export the entire public API from oz-crypto.
pub use oz_crypto::*;
