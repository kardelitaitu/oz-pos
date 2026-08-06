//! Tax Service — tax calculation and rate management workflows.

use crate::models::TaxRate;
use crate::repository::TaxRepository;
use rusqlite::Connection;

/// Service encapsulating tax workflows.
pub struct TaxService;

impl TaxService {
    /// Retrieve tax rate by ID.
    pub fn get_tax_rate(conn: &Connection, id: &str) -> Result<Option<TaxRate>, anyhow::Error> {
        let repo = TaxRepository::new(conn);
        repo.get_tax_rate(id)
    }

    /// List all active tax rates, ordered by name.
    ///
    /// TAX-03: filters `is_active = 1` exactly like
    /// `oz_core::db::Store::list_tax_rates`, so archived (immutable)
    /// rates stay hidden through the module boundary. Cross-layer parity
    /// is pinned by `modules/tax/tests/boundary_contract.rs`.
    pub fn list_tax_rates(conn: &Connection) -> Result<Vec<TaxRate>, anyhow::Error> {
        let repo = TaxRepository::new(conn);
        repo.list_tax_rates()
    }
}
