//! EDC terminal configuration CRUD — PLANNED (stubs).
//!
//! These methods are stubs until the EDC payment terminal feature is
//! implemented. The `edc_terminals` table is created by migration
//! `20260824_media_edc.sql` but the Rust methods below are not yet
//! functional.

use super::Store;
use crate::error::CoreError;

/// PLANNED: register a new EDC terminal.
pub fn create_edc_terminal(_store: &Store<'_>) -> Result<(), CoreError> {
    Err(CoreError::Internal(
        "create_edc_terminal — PLANNED, not implemented yet".into(),
    ))
}

/// PLANNED: list all configured EDC terminals.
pub fn list_edc_terminals(_store: &Store<'_>) -> Result<Vec<EdcTerminalConfig>, CoreError> {
    Err(CoreError::Internal(
        "list_edc_terminals — PLANNED, not implemented yet".into(),
    ))
}

/// PLANNED: update an EDC terminal configuration.
pub fn update_edc_terminal(_store: &Store<'_>) -> Result<(), CoreError> {
    Err(CoreError::Internal(
        "update_edc_terminal — PLANNED, not implemented yet".into(),
    ))
}

/// PLANNED: delete an EDC terminal configuration.
pub fn delete_edc_terminal(_store: &Store<'_>) -> Result<(), CoreError> {
    Err(CoreError::Internal(
        "delete_edc_terminal — PLANNED, not implemented yet".into(),
    ))
}

/// An EDC terminal configuration row (from `edc_terminals`).
#[derive(Debug, Clone)]
pub struct EdcTerminalConfig {
    /// UUID v7.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Connection type: "wired" or "wireless".
    pub connection_type: String,
    /// Transport: "serial", "usb", "bluetooth", "tcp".
    pub transport: String,
    /// Device path / MAC address / host:port.
    pub address: String,
    /// Vendor (e.g. "ingenico", "verifone", "pax").
    pub vendor: Option<String>,
    /// Model identifier.
    pub model: Option<String>,
    /// Whether the terminal is active.
    pub is_active: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
}
