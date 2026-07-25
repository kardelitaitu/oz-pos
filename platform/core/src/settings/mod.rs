//! Settings store — typed access to a key-value settings table.
//!
//! The [`Settings`] struct provides read/write helpers for a generic
//! `settings` table (`key TEXT PRIMARY KEY, value TEXT`). All methods
//! take a `&rusqlite::Connection` so callers control transaction
//! boundaries.

/// Typed access to a key-value `settings` table.
pub struct Settings;

mod raw;
mod typed;

pub mod keys;

#[cfg(test)]
mod split_tests;

/// Test-only helpers shared across `settings` test modules.
#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tests;
