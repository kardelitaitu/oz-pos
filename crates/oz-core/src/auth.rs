//! Authentication utilities — delegates to `platform-core`.
//!
//! Re-exports the hash/verify functions and `LoginSession` from the
//! platform-core auth module.

pub use platform_core::auth::{hash_pin, verify_pin, LoginSession};
