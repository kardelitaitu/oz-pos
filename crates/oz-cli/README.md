# oz-cli

CLI tool for OZ-POS maintenance — migrations, backup, export, and data CRUD.

## Subcommands

| Command | Description |
|---------|-------------|
| `oz migrate` | Apply pending SQL migrations |
| `oz init-db [--preset <preset>]` | Seed DB with settings, feature presets (simple-retail, restaurant, full-store, custom), 39 currencies, 3 default roles, and an admin user |
| `oz product list\|get\|create\|update\|delete` | Full product CRUD (SKU, name, price in minor units, category, barcode) |
| `oz category list\|get\|create\|delete` | Category CRUD (id, name, hex colour) |
| `oz inventory get\|adjust` | Stock query by SKU; signed-delta adjustment |
| `oz sale list\|get [--format text\|json]\|update-status` | Sale listing, detail view, status transitions (pending, active, completed, voided) |
| `oz customer list\|get\|create` | Customer CRUD (name, email, phone, notes) |
| `oz user list\|get\|create` | User CRUD (username, pin_hash, display_name, role_id) |
| `oz backup --output <path>` | Online SQLite backup to a file |
| `oz restore --input <path>` | Restore DB from a backup file (file copy) |
| `oz export <daily-summary\|sales-by-hour>` | CSV report written to stdout |
| `oz export-ozpkg --output <path> --password <pw>` | Encrypted `.ozpkg` export (Argon2id + AES-256-GCM); `--types` selects data kinds |
| `oz import-ozpkg --input <path> --password <pw>` | Decrypt and inspect a `.ozpkg` file; `--dry-run` reads metadata without writing |
| `oz --version` | Print version |

## Notes

- DB path defaults to `./oz-pos.db`; use `--db <path>` (global flag) to override.
- Prices and monetary values are `i64` minor units (e.g. `350` for $3.50).
- `oz import-ozpkg` currently supports dry-run inspection only; write logic is pending.

> last audited 28-06-26 by docs-auditor
