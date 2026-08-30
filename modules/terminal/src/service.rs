/*
last audited 25-07-26 by RSA-Agent (modules-terminal slice A: service verified)
crate: modules-terminal | status: SAFE | lint: CLEAN
findings: clean thin service facade
next: none | perf: N/A
*/
//! Terminal Service — POS terminal business logic.

use crate::error::TerminalError;
use crate::models::Terminal;
use crate::repository::TerminalRepository;
use rusqlite::Connection;

/// Service encapsulating terminal business workflows.
pub struct TerminalService;

impl TerminalService {
    /// Retrieve terminal by ID.
    pub fn get_terminal(conn: &Connection, id: &str) -> Result<Option<Terminal>, TerminalError> {
        let repo = TerminalRepository::new(conn);
        repo.get_terminal(id)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
