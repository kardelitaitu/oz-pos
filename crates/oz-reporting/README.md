# oz-reporting

Analytics and CSV export engine for OZ-POS. Aggregates data from the local SQLite store and produces daily summaries, sales-by-hour, inventory movement, and CSV exports. Computed on-device to keep the offline-first guarantee; cloud sync of pre-aggregated reports is a separate service.

## Public API

- [`ReportingError`](src/error.rs) — `thiserror`-based error for reporting queries and exports.

## Planned surface

- `daily_summary(date)` — sales, refunds, net, top SKUs, payment mix.
- `sales_by_hour(window)` — hourly bucketed sales totals.
- `inventory_movement(window)` — stock-in / stock-out per SKU.
- `export_csv(kind, path)` — synchronous export of any report to a CSV file.

## Status

Scaffold only. Reports are added once the cart, sale, payment, and inventory tables stabilize.
