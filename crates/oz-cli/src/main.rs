//! `oz` — OZ-POS command-line tool.
//!
//! Subcommands are added as the framework grows. This scaffold
//! defines the clap-derived argument struct, a `--version`/`--help`
//! surface, and a placeholder `migrate` subcommand that will call
//! into the migration runner once `oz-core` exposes it.

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};

/// OZ-POS command-line tool.
#[derive(Debug, Parser)]
#[command(name = "oz", version, about = "OZ-POS maintenance and migration CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Apply pending SQL migrations to the local database.
    Migrate,
    /// Snapshot the local SQLite store to a backup file.
    Backup {
        /// Destination path for the backup file.
        #[arg(short, long)]
        output: String,
    },
    /// Write a CSV report for the given time window.
    Export {
        /// Report kind (e.g. `daily-summary`, `sales-by-hour`).
        kind: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Migrate) => run_migrate().context("running migrations"),
        Some(Command::Backup { output }) => {
            println!("backup -> {output}: not yet implemented (scaffold)");
            Ok(())
        }
        Some(Command::Export { kind }) => {
            println!("export {kind}: not yet implemented (scaffold)");
            Ok(())
        }
        None => {
            // No subcommand: print help and exit cleanly.
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            Ok(())
        }
    }
}

fn run_migrate() -> Result<()> {
    use oz_core::migrations;
    use rusqlite::Connection;

    // The CLI defaults to the local SQLite store at ./oz-pos.db. Pass
    // --db <path> to override (TODO once we add a global flag).
    let path = std::env::var("OZ_POS_DB").unwrap_or_else(|_| "oz-pos.db".to_owned());
    eprintln!("opening {path}");
    let mut conn =
        Connection::open(&path).with_context(|| format!("opening database at {path}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enabling foreign_keys")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("enabling WAL")?;
    migrations::run(&mut conn).context("applying migrations")?;
    eprintln!("migrations up to date");
    Ok(())
}
