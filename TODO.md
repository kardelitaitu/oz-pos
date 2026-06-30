# TODO — OZ-POS Phase 1 MVP

## ✅ Completed

- **Barcode scanning** — USB HID scanner driver → product lookup → add-to-cart (useBarcodeScanner hook, scanner lifecycle, barcode:scanned events)
- **User / Staff management** — StaffManagementScreen (list, add, edit, deactivate/reactivate, Fluent-localized) + 6 unit tests
- **Inventory adjustments** — InventoryAdjustmentScreen (product search, add/remove stock, reason dropdown, validation, Fluent-localized) + 6 unit tests
- **Dashboard charts** — horizontal bar chart replacing plain table in SalesDashboardScreen + 6 unit tests
- **Receipt printing** — ESC/POS formatting, Tauri command, end-to-end from PaymentModal and SalesHistoryScreen reprint
- **Tax configuration** — TaxConfigurationScreen (CRUD, basis-point input, is_default exclusivity) + 6 unit tests
- **Sales history & dashboard** — SalesHistoryScreen (table + detail modal + reprint), SalesDashboardScreen (KPI cards + hourly bar chart)
- **Payment modal** — PaymentModal (cash/card/other, tendered input, change preview, sale completion)
- **Product CRUD** — ProductManagementScreen (table + add/edit/delete modal)
- **Store & receipt settings** — Settings page (currency toggle, decimal separator, paper width, footer, store name/address/tax ID)
- **Sales pipeline persistence** — Sale::from_cart() → Status::Pending → complete_sale transition
- **Tax assignment** — migration 012 (product_taxes junction table), Store methods, Tauri commands, product form tax-rate checkboxes
- **Currency configuration** — list_currencies command, default currency dropdown on Settings page
- **Exchange rate management** — UI for exchange_rates table (ExchangeRateScreen with CRUD, Fluent-localized)
- **User ID on sales** — Migration 014 adds `user_id` column; fully wired through `Sale::from_cart_with_user`, `complete_sale` command, `Store::create_sale`
- **Active cashier session** — `userId` is required in `completeSale`; POS screen gated behind login; lock button in cart panel header
- **Customer Management** — backend CRUD + Tauri commands + full screen at `ui/src/features/customers/`
- **Product variants** — migration 015, domain types, Store methods, Tauri commands, variant management UI (modal in ProductManagementScreen)
- **Multi-terminal support** — migration 016, terminal registration, Store methods, Tauri commands, management UI
- **Offline queue** — migration 018, transaction queue for later sync, Store methods, Tauri commands, management UI
- **Cloud sync** — sync settings in SettingsPage, background daemon (30s interval), Tauri commands (`trigger_sync`, `get_sync_settings`, `update_sync_settings`), `sync_client` module with config + pending sync
- **Tests** — 460+ tests across all crates, all passing, clippy-clean

## Medium-term

- **Refund / return flow** — partial or full refund of completed sales
- **Shift management** — open/close shift with cash reconciliation
- **Staff RBAC** — role-based permission gating on screens/actions
