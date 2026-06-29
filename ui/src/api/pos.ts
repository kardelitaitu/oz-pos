// ── api/pos.ts has been split into domain files ──
// Import from the appropriate domain module instead:
//
//   api/sales.ts       — POS cart, history, void, refunds, holds, dashboard, receipts
//   api/products.ts    — Products CRUD, variants, categories, barcode lookup, stock
//   api/tax.ts         — Tax rates, category-tax assignments
//   api/settings.ts    — Store/receipt settings, setup wizard, feature flags
//   api/staff.ts       — Login, staff CRUD, roles
//   api/customers.ts   — Customer CRUD
//   api/currency.ts    — Currencies, exchange rates
//   api/hardware.ts    — Barcode scanner, cash drawer, receipt printer
//   api/terminals.ts   — Terminal registration and management
//   api/offline.ts     — Offline queue, cloud sync
//   api/audit.ts       — Audit log
//   api/system.ts      — Ping, version info
