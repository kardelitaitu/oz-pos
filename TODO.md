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
- **Tests** — 24 new tests across 4 screen test files (122 total, all passing)
- **Audit Log, Orders, Feature Toggle** — screens already built and wired with routes + nav items

## WIP / Customer Management

- Backend CRUD & Tauri commands exist. Screen exists at `ui/src/features/customers/CustomerManagementScreen.tsx` but marked WIP — not needed for MVP.

## Medium-term

- **Multi-terminal support** — terminal registration + sync
- **Offline mode** — queue transactions when network is down
- **Cloud sync** — push sales to remote server
- **Exchange rate management** — UI for exchange_rates table
- **Product variants** — size/color/flavor variants per product
