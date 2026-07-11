//! Database infrastructure — migration runner, connection pool,
//! and store-scoped database manager (ADR #4 Phase 2).

pub mod manager;
pub mod migrations;
pub mod pool;

pub use manager::StoreDatabaseManager;
pub use migrations::{Migration, run};
pub use pool::Pool;
