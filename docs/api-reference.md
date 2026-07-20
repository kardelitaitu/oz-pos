# API Reference — OZ-POS

All commands are Tauri IPC commands invoked from the React frontend via `ui/src/api/*.ts`. Returns `Result<T, AppError>`.

## Auth

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `check_login` | `username: String, pin: String` | `Session` | Authenticate user with username + PIN |
| `logout` | `session_token: String` | `()` | End current session |
| `check_session` | `session_token: String` | `Session` | Validate session is still active |
| `list_roles` | — | `Vec<Role>` | List available staff roles |

## POS & Sales

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_products` | `location_id: Option<String>` | `Vec<Product>` | List products for current location |
| `search_products` | `query: String` | `Vec<Product>` | Search by name, SKU, or barcode |
| `create_sale` | `cart: Cart, payments: Vec<Payment>` | `SaleResult` | Complete a sale with payments |
| `void_sale` | `sale_id: String, reason: String` | `()` | Void a completed sale |
| `refund_sale` | `sale_id: String, amount: Money` | `()` | Issue a partial or full refund |
| `list_sales` | `start: String, end: String` | `Vec<SaleListItem>` | List sales in date range |
| `get_sale` | `sale_id: String` | `SaleDetail` | Get full sale with line items |

## Products & Inventory

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `create_product` | `sku, name, price, ...` | `Product` | Create a new product |
| `update_product` | `product_id, name, price, ...` | `Product` | Update existing product |
| `delete_product` | `product_id: String` | `()` | Soft-delete a product |
| `create_stock_count` | `location_id, items` | `StockCount` | Start a stock count session |
| `create_stock_transfer` | `from, to, items` | `Transfer` | Transfer stock between locations |
| `list_low_stock` | `threshold: i64` | `Vec<Product>` | Products below threshold |

## Settings

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_store_settings` | — | `StoreSettings` | Store name, currency, timezone |
| `set_store_settings` | `settings: StoreSettings` | `()` | Update store configuration |
| `get_receipt_settings` | — | `ReceiptSettings` | Receipt header, footer, tax display |
| `set_receipt_settings` | `settings: ReceiptSettings` | `()` | Update receipt configuration |
| `get_setting` | `key: String` | `Option<String>` | Get any setting by key |
| `set_setting` | `key: String, value: String` | `()` | Set any setting by key |

## Reporting

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_daily_revenue` | `start: String, end: String` | `Vec<DailyRevenue>` | Revenue grouped by day |
| `get_weekly_revenue` | `start: String, end: String` | `Vec<WeeklyRevenue>` | Revenue grouped by week |
| `get_top_products` | `start, end, limit` | `Vec<TopProduct>` | Best-selling products |
| `get_hourly_heatmap` | `start: String, end: String` | `Vec<HourlyHeatmap>` | Sales by hour |
| `export_eod_report` | `date: String` | `EodReport` | End-of-day summary |
| `build_custom_report` | `dataset, columns, dates` | `CustomReportResponse` | Custom column report |

## Shifts & Cash

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `open_shift` | `opening_balance: Money` | `Shift` | Open a new shift |
| `close_shift` | `shift_id: String` | `ShiftSummary` | Close with sales summary |
| `get_active_shift` | — | `Option<Shift>` | Get currently open shift |
| `list_shifts` | — | `Vec<Shift>` | List all shifts |
| `create_cash_payout` | `amount, reason` | `()` | Record cash removal |

## Sync & Offline

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `get_sync_status` | — | `SyncStatus` | Connection status + queue depth |
| `list_offline_items` | — | `Vec<OfflineItem>` | Pending sync queue items |
| `retry_sync` | — | `SyncResult` | Force retry pending syncs |
| `import_snapshot` | `snapshot: Snapshot` | `ImportResult` | Bulk import data |

## Hardware

| Command | Parameters | Returns | Description |
|---------|-----------|---------|-------------|
| `discover_hardware` | — | `Vec<Device>` | Auto-discover connected devices |
| `print_receipt` | `receipt_data: String` | `()` | Send receipt to printer |
| `get_printer_status` | — | `PrinterStatus` | Paper level, errors |
| `open_cash_drawer` | — | `()` | Open connected cash drawer |
