# oz-cli

Command-line tools for OZ-POS — migrations, backup, export, smoke tests. The `oz` binary is the maintenance surface a merchant or operator runs from a terminal.

## Subcommands

- `oz migrate` — apply pending SQL migrations to the local database
- `oz backup --output <path>` — snapshot the local SQLite store
- `oz export <kind>` — write a CSV report (`daily-summary`, `sales-by-hour`, ...)
- `oz --help` / `oz --version`

## Public API (library)

- [`CliError`](src/error.rs) — `thiserror`-based error type, shared by `main.rs` and the subcommand modules.

## Example

```bash
# Show version
oz --version

# Apply migrations
oz migrate

# Snapshot the local DB
oz backup --output /var/backups/oz-$(date +%F).db

# Export a daily report
oz export daily-summary
```

## Status

Scaffold only. Subcommands return `not yet implemented (scaffold)` until the corresponding crate lands its real implementation.
