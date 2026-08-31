/*
last audited 25-07-26 by RSA-Agent (modules-settings slice A: service verified)
crate: modules-settings | status: SAFE | lint: CLEAN
findings: clean thin service facade (see MSL-5 note on repository)
next: none | perf: N/A
*/
//! Settings Service — configuration business logic.

use crate::error::SettingsError;
use crate::repository::SettingsRepository;
use rusqlite::Connection;

/// Service encapsulating settings workflows.
pub struct SettingsService;

impl SettingsService {
    /// Retrieve setting by key.
    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, SettingsError> {
        let repo = SettingsRepository::new(conn);
        repo.get(key)
    }

    /// Set setting by key.
    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), SettingsError> {
        let repo = SettingsRepository::new(conn);
        repo.set(key, value)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
