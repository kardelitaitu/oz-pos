//! Payment gateway configuration CRUD — PLANNED (stubs).
//!
//! These methods are stubs until the payment gateway feature is fully
//! implemented. The `payment_gateways` table is created by migration
//! `20260825_payment_infra.sql` but the Rust methods below are not yet
//! functional.

use super::Store;
use crate::error::CoreError;

/// PLANNED: upsert a gateway configuration for a tenant.
pub fn upsert_gateway(_store: &Store<'_>) -> Result<(), CoreError> {
    Err(CoreError::Internal(
        "upsert_gateway — PLANNED, not implemented yet".into(),
    ))
}

/// PLANNED: list active gateways for a tenant.
pub fn list_active_gateways(_store: &Store<'_>) -> Result<Vec<PaymentGatewayConfig>, CoreError> {
    Err(CoreError::Internal(
        "list_active_gateways — PLANNED, not implemented yet".into(),
    ))
}

/// PLANNED: load a single gateway config by name.
pub fn get_gateway(_store: &Store<'_>) -> Result<Option<PaymentGatewayConfig>, CoreError> {
    Err(CoreError::Internal(
        "get_gateway — PLANNED, not implemented yet".into(),
    ))
}

/// A payment gateway configuration row (from `payment_gateways`).
#[derive(Debug, Clone)]
pub struct PaymentGatewayConfig {
    /// UUID v7.
    pub id: String,
    /// RLS tenant scope.
    pub tenant_id: String,
    /// Gateway name: "stripe", "square", "midtrans", "paddle".
    pub name: String,
    /// Whether the gateway is enabled.
    pub is_active: bool,
    /// Gateway-specific keys (api key, sandbox flag, ...).
    pub config_json: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
}
