/*
last audited 25-07-26 by RSA-Agent
crate: oz-payment | status: SAFE | lint: CLEAN
findings: RwLock catalogue sound; build_from_config PLANNED stub fails closed (PAY-12); "config change not code change" promise not yet real — drivers constructed directly by callers
next: implement build_from_config when registry wiring lands | perf: N/A
*/
//! Payment processor registry — PLANNED (stub).
//!
//! The runtime catalogue of available payment processors, mirroring
//! `oz_hal::DriverRegistry`. Commands reach a processor through the
//! registry (e.g. `registry.processor("stripe")`) and never construct a
//! specific driver directly, so switching gateways is a config change
//! (see the `payment_gateways` table), not a code change.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::PaymentProcessor;
use crate::error::PaymentError;

/// Shared, mutable catalogue of payment processors.
#[derive(Default)]
pub struct PaymentProcessorRegistry {
    processors: RwLock<HashMap<String, Arc<dyn PaymentProcessor>>>,
}

impl PaymentProcessorRegistry {
    /// Construct an empty registry. Use [`Self::register`] to add drivers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a processor under `name` (e.g. `"stripe"`). Overwrites
    /// any previous entry with the same name.
    pub async fn register(&self, name: &str, processor: Arc<dyn PaymentProcessor>) {
        self.processors
            .write()
            .await
            .insert(name.to_owned(), processor);
    }

    /// Look up a processor by name. Returns `None` if not registered.
    pub async fn processor(&self, name: &str) -> Option<Arc<dyn PaymentProcessor>> {
        self.processors.read().await.get(name).cloned()
    }

    /// Snapshot of registered processor names.
    pub async fn processor_names(&self) -> Vec<String> {
        self.processors.read().await.keys().cloned().collect()
    }

    /// Build a processor from a gateway configuration.
    ///
    /// **PLANNED:** currently returns [`PaymentError::Unsupported`] for
    /// every gateway. The real implementation will read the gateway's
    /// `config_json` (api key, sandbox flag) and construct the matching
    /// driver (Stripe, Square, Midtrans/QRIS, Paddle, or an EDC terminal).
    pub async fn build_from_config(
        &self,
        name: &str,
    ) -> Result<Arc<dyn PaymentProcessor>, PaymentError> {
        let _ = name;
        Err(PaymentError::Unsupported(
            "PaymentProcessorRegistry::build_from_config — PLANNED, not implemented yet".into(),
        ))
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
