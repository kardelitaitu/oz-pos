//! Terminal Service — POS terminal business logic.

use crate::models::Terminal;
use crate::repository::TerminalRepository;
use rusqlite::Connection;

/// Service encapsulating terminal business workflows.
pub struct TerminalService;

impl TerminalService {
    /// Retrieve terminal by ID.
    pub fn get_terminal(conn: &Connection, id: &str) -> Result<Option<Terminal>, anyhow::Error> {
        let repo = TerminalRepository::new(conn);
        repo.get_terminal(id)
    }
}
