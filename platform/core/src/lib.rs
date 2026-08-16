#![warn(missing_docs)]

//! Platform Core — shared infrastructure for OZ-POS.
//!
//! This crate provides reusable infrastructure services that are
//! consumed by all other crates and modules in the OZ-POS workspace:
//!
//! - [`database`] — migration runner and connection pool
//! - [`auth`] — PIN hashing, verification, and login session types
//! - [`rbac`] — Role-Based Access Control primitives (Role, Permission)
//! - [`permission_registry`] — code-resident permission registry with
//!   write-time grant validation (ADR #35 D3 / spec 0046)
//! - [`settings`] — generic key-value settings store with typed helpers
//! - [`error`] — shared error type ([`PlatformError`])

pub mod auth;
pub mod database;
pub mod error;
pub mod permission_registry;
pub mod rbac;
pub mod settings;
pub mod terminal_profile;

pub use database::StoreDatabaseManager;
pub use error::PlatformError;
