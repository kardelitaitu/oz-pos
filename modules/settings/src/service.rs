//! Settings Service — configuration business logic.

use crate::repository::SettingsRepository;
use rusqlite::Connection;

/// Service encapsulating settings workflows.
pub struct SettingsService;

impl SettingsService {
    /// Retrieve setting by key.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, anyhow::Error> {
        let repo = SettingsRepository::new(conn);
        repo.get(key)
    }

    /// Set setting by key.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), anyhow::Error> {
        let repo = SettingsRepository::new(conn);
        repo.set(key, value)
    }
}
