//! Staff Service — user and role business workflows.

use crate::models::{Role, User};
use crate::repository::StaffRepository;
use rusqlite::Connection;

/// Service encapsulating staff management workflows.
pub struct StaffService;

impl StaffService {
    /// Retrieve user by ID.
    pub fn get_user(conn: &Connection, id: &str) -> Result<Option<User>, anyhow::Error> {
        let repo = StaffRepository::new(conn);
        repo.get_user(id)
    }

    /// Retrieve role by ID.
    pub fn get_role(conn: &Connection, id: &str) -> Result<Option<Role>, anyhow::Error> {
        let repo = StaffRepository::new(conn);
        repo.get_role(id)
    }
}
