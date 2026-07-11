//! Platform Core — shared infrastructure for OZ-POS.
//!
//! This crate provides reusable infrastructure services that are
//! consumed by all other crates and modules in the OZ-POS workspace:
//!
//! - [`database`] — migration runner and connection pool
//! - [`auth`] — PIN hashing, verification, and login session types
//! - [`rbac`] — Role-Based Access Control primitives (Role, Permission)
//! - [`settings`] — generic key-value settings store with typed helpers
//! - [`error`] — shared error type ([`PlatformError`])

pub mod auth;
pub mod database;
pub mod error;
pub mod rbac;
pub mod settings;

pub use database::StoreDatabaseManager;
pub use error::PlatformError;
