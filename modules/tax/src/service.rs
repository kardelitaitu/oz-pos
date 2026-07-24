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
}
