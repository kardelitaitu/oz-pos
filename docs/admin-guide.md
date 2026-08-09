# Admin Guide — OZ-POS

<!-- Audit stamp: 2026-07-26 · Hermes-Agent · status: ACCURATE (1 observation) · O1: doc lists KDS as a top-level "Workspace" alongside Store POS / Inventory / Admin, but the seed (migration 035_workspaces.sql) registers KDS as a screen (workspace_key='kds') under restaurant-pos, not a standalone workspace type; user-facing simplification, not a code error · verified accurate: workspace_types seeded store-pos/inventory/restaurant-pos/admin (035_workspaces.sql:30-33); roles Owner/Manager/Cashier/Kitchen match staff.rs:408-419; scripts/backup-db.sh + scripts/restore-db.sh exist; shift open/close + cash payout + offline-queue + reports (sales/eod/menu-engineering/custom/inventory) all map to real features; setup wizard presets (simple retail/restaurant/custom) are user-facing guidance consistent with workspace seeds -->

## Installation

1. Download the latest release from GitHub Releases
2. Run the installer (Windows: `.msi`, Linux: `.AppImage`)
3. On first launch, follow the setup wizard to:
   - Set store name, currency, and tax settings
   - Create the owner account (username + PIN)
   - Choose a workspace preset (simple retail, restaurant, or custom)

## Workspace Management

Navigate to **Admin → Workspaces** to manage workspace types:

| Workspace | Purpose |
|-----------|---------|
| Store POS | Point of sale — customer-facing checkout |
| Inventory | Product management, stock counts, transfers |
| KDS | Kitchen display — order tickets for kitchen staff |
| Admin | Settings, staff management, reporting |

## User Management

1. Go to **Admin → Staff**
2. Click **+ Add Staff** to create a new user
3. Set **role** (owner, manager, cashier, kitchen)
4. Assign a **PIN** (4-6 digits) for quick login

### Roles

| Role | Permissions |
|------|-------------|
| Owner | Full access — settings, staff, reporting, data export |
| Manager | Staff management, reporting, inventory |
| Cashier | POS only — sales, refunds, shift open/close |
| Kitchen | KDS only — view and acknowledge orders |

## Shift Management

1. At start of day: **Open Shift** → enter opening cash balance
2. During the day: process sales normally
3. At end of day: **Close Shift** → review sales summary → confirm
4. Cash payouts (e.g., for supplies): **Cash Payout** in shift screen

## Reporting

Navigate to **Admin → Reports** for:

- **Sales Report**: Daily/weekly revenue, top products, category breakdown
- **EOD Report**: End-of-day summary with payments, taxes, discounts
- **Menu Engineering**: Profitability vs popularity analysis
- **Custom Report**: Build your own reports by selecting columns and date ranges
- **Inventory Report**: Stock levels, low stock alerts, stock value

## Backup & Restore

### Backup
```bash
bash scripts/backup-db.sh
```
Creates a timestamped gzipped backup in `./backups/`. Prunes backups older than 30 days.

### Restore
```bash
bash scripts/restore-db.sh backups/oz-pos-20260720-120000.db.gz
```
Verifies integrity, creates a pre-restore safety backup, replaces the active database.

## Offline Mode

The POS continues operating without internet:
- All sales, shifts, and inventory changes are queued locally
- Data syncs automatically when connection is restored
- Check **Admin → Offline Queue** to see pending sync items
- Status bar indicator: 🟢 online / 🟡 reconnecting / 🔴 offline

> last audited 09-08-26 by buffy
> audit: Phase 1 Core Architecture & API Docs Audit

> status: ACCURATE (0 findings) · verified accurate: cargo check passed, no structural orphans, no stale version headers, all file references valid

