# API Reference — OZ-POS

> **Note:** This document lists all Tauri IPC commands available from the React frontend via `ui/src/api/*.ts`. All commands return `Result<T, AppError>`. Commands are organized by feature module.

## Authentication & Sessions

| `create_session` | Create a new session and return an opaque session token.

ADR #4 / ADR #7: Called after login + workspace selection to |
| `destroy_session` | Destroy an active session, invalidating the token.

ADR #4 / ADR #7: Called on logout or store switch. After this |
| `staff_check_username` | Check a username before the PIN step (STAFF-06).

Returns a **uniform** pre-auth response so the command cannot be used as |
| `staff_login` | Authenticate a staff member by username and PIN.

Looks up the user by username, verifies the PIN against the stored |

## Hardware and Peripherals

| `assign_table_order` | Assign an order/sale to a table.

**Deprecated for multi-store (ADR #7):** Use `assign_table_order_scoped`. |
| `assign_table_order_scoped` | Assign an order to a table in the store resolved from a session token. ADR #7. |
| `clear_device_binding` | Clear a terminal's device binding.

**Deprecated for multi-store (ADR #7):** Use `clear_device_binding_scoped`. |
| `clear_device_binding_scoped` | Clear a device binding in the store resolved from a session token. ADR #7. |
| `create_table` | Create a new table.

**Deprecated for multi-store (ADR #7):** Use `create_table_scoped`. |
| `create_table_scoped` | Create a table in the store resolved from a session token. ADR #7. |
| `delete_table` | Delete a table by id.

**Deprecated for multi-store (ADR #7):** Use `delete_table_scoped`. |
| `delete_table_scoped` | Delete a table in the store resolved from a session token. ADR #7. |
| `delete_terminal` | Delete a terminal by id.

**Deprecated for multi-store (ADR #7):** Use `delete_terminal_scoped`. |
| `delete_terminal_override` | Delete a feature override for a terminal.

**Deprecated for multi-store (ADR #7):** Use `delete_terminal_override_scoped`. |
| `delete_terminal_override_scoped` | Delete a terminal override in the store resolved from a session token. ADR #7. |
| `delete_terminal_profile` | Delete a terminal's profile.

**Deprecated for multi-store (ADR #7):** Use `delete_terminal_profile_scoped`. |
| `delete_terminal_profile_scoped` | Delete a terminal profile in the store resolved from a session token. ADR #7. |
| `delete_terminal_scoped` | Delete a terminal in the store resolved from a session token. ADR #7. |
| `discover_hardware` | Discover all connected USB hardware devices (scanners, printers, scales).

Calls `oz_hal::transport::usb::probe_all()` to enumerate known USB |
| `display_clear` | Clear a customer-facing pole display. |
| `display_show` | Show content on a customer-facing pole display. |
| `get_device_binding` | Get a terminal's device binding and validate its HMAC signature.

**Deprecated for multi-store (ADR #7):** Use `get_device_binding_scoped`. |
| `get_device_binding_scoped` | Get device binding from the store resolved from a session token. ADR #7. |
| `get_table` | Get a single table by id.

**Deprecated for multi-store (ADR #7):** Use `get_table_scoped`. |
| `get_table_scoped` | Get a table from the store resolved from a session token. ADR #7. |
| `get_terminal` | Get a single terminal by id.

**Deprecated for multi-store (ADR #7):** Use `get_terminal_scoped`. |
| `get_terminal_profile` | Get the profile for a terminal.

**Deprecated for multi-store (ADR #7):** Use `get_terminal_profile_scoped`. |
| `get_terminal_profile_scoped` | Get a terminal profile from the store resolved from a session token. ADR #7. |
| `get_terminal_scoped` | Get a terminal from the store resolved from a session token. ADR #7. |
| `list_displays` | List all registered customer displays. |
| `list_scanners` | List all registered barcode scanners. |
| `list_sections` | List all section names.

**Deprecated for multi-store (ADR #7):** Use `list_sections_scoped`. |
| `list_sections_scoped` | List sections for the store resolved from a session token. ADR #7. |
| `list_tables` | List tables, optionally filtered by section.

**Deprecated for multi-store (ADR #7):** Use `list_tables_scoped`. |
| `list_tables_scoped` | List tables for the store resolved from a session token. ADR #7. |
| `list_terminal_overrides` | List feature overrides for a terminal.

**Deprecated for multi-store (ADR #7):** Use `list_terminal_overrides_scoped`. |
| `list_terminal_overrides_scoped` | List terminal overrides from the store resolved from a session token. ADR #7. |
| `list_terminal_profiles` | List all terminal profiles.

**Deprecated for multi-store (ADR #7):** Use `list_terminal_profiles_scoped`. |
| `list_terminal_profiles_scoped` | List terminal profiles from the store resolved from a session token. ADR #7. |
| `list_terminals` | List all registered terminals.

**Deprecated for multi-store (ADR #7):** Use `list_terminals_scoped`. |
| `list_terminals_scoped` | List terminals from the store resolved from a session token. ADR #7. |
| `ping_terminal` | Ping a terminal to update its last_seen_at timestamp.

**Deprecated for multi-store (ADR #7):** Use `ping_terminal_scoped`. |
| `ping_terminal_scoped` | Ping a terminal in the store resolved from a session token. ADR #7. |
| `print_sales_receipt_scoped` | Print sales receipt for the store resolved from a session token. ADR #7.
Settings (store name, address, receipt config) are loaded from the
store-scop... |
| `register_terminal` | Register a new terminal.

**Deprecated for multi-store (ADR #7):** Use `register_terminal_scoped`. |
| `register_terminal_scoped` | Register a terminal in the store resolved from a session token. ADR #7. |
| `release_table` | Release a table (clear its order assignment).

**Deprecated for multi-store (ADR #7):** Use `release_table_scoped`. |
| `release_table_scoped` | Release a table in the store resolved from a session token. ADR #7. |
| `set_device_binding` | Set (or update) a terminal's device binding with HMAC signature.

**Deprecated for multi-store (ADR #7):** Use `set_device_binding_scoped`. |
| `set_device_binding_scoped` | Set a device binding in the store resolved from a session token. ADR #7. |
| `set_terminal_override` | Set (upsert) a feature override for a terminal.

**Deprecated for multi-store (ADR #7):** Use `set_terminal_override_scoped`. |
| `set_terminal_override_scoped` | Set a terminal override in the store resolved from a session token. ADR #7. |
| `set_terminal_profile` | Set (upsert) the profile for a terminal.

**Deprecated for multi-store (ADR #7):** Use `set_terminal_profile_scoped`. |
| `set_terminal_profile_scoped` | Set a terminal profile in the store resolved from a session token. ADR #7. |
| `start_scanner` | Start a background polling task for the named scanner.

Every decoded barcode is emitted as a `barcode:scanned` event |
| `stop_scanner` | Stop the active barcode scanner background task (if any). |
| `update_table` | Update an existing table.

**Deprecated for multi-store (ADR #7):** Use `update_table_scoped`. |
| `update_table_scoped` | Update a table in the store resolved from a session token. ADR #7. |
| `update_table_status` | Update a table's status (e.g. "occupied", "available").

**Deprecated for multi-store (ADR #7):** Use `update_table_status_scoped`. |
| `update_table_status_scoped` | Update a table's status in the store resolved from a session token. ADR #7. |
| `update_terminal` | Update an existing terminal.

**Deprecated for multi-store (ADR #7):** Use `update_terminal_scoped`. |
| `update_terminal_scoped` | Update a terminal in the store resolved from a session token. ADR #7. |

## KDS (Kitchen Display System)

| `create_kds_order_from_sale` | Create KDS orders from a completed sale. Returns one order per kitchen zone.

**Deprecated for multi-store (ADR #7):** Use `create_kds_order_from_sale... |
| `create_kds_order_from_sale_scoped` | Create KDS orders in the store resolved from a session token. ADR #7.

Passes the session's `store_id` so the KDS order carries store identity |
| `get_kds_order` | Get a KDS order by id from the global database.

**Deprecated for multi-store (ADR #7):** Use `get_kds_order_scoped`. |
| `get_kds_order_lines_scoped` | Get all line items for a KDS order (scoped — ADR #7).

Returns structured line items with course and modifier data, |
| `get_kds_order_scoped` | Get a KDS order from the store resolved from a session token. ADR #7. |
| `get_kds_queue` | Get the kitchen queue from the global database.

**Deprecated for multi-store (ADR #7):** Use `get_kds_queue_scoped`. |
| `get_kds_queue_scoped` | Get the kitchen queue for the store resolved from a session token. ADR #7. |
| `list_kds_orders` | List KDS orders from the global database.

**Deprecated for multi-store (ADR #7):** Use `list_kds_orders_scoped`. |
| `list_kds_orders_scoped` | List KDS orders for the store resolved from a session token. ADR #7. |
| `print_kds_chit_scoped` | Print a kitchen chit for a specific KDS order by ID (scoped — ADR #7).

Useful for manual re-print from the KDS screen when a chit was lost |
| `update_kds_line_item_status_scoped` | Update the status of a single KDS line item in the store resolved
from a session token. ADR #7.
 |
| `update_kds_order_items` | Update the items (summary + count) on an existing KDS order.

**Deprecated for multi-store (ADR #7):** Use `update_kds_order_items_scoped`. |
| `update_kds_order_items_scoped` | Update the items on a KDS order in the store resolved from a session token. ADR #7. |
| `update_kds_status` | Update a KDS order's status in the global database.

**Deprecated for multi-store (ADR #7):** Use `update_kds_status_scoped`. |
| `update_kds_status_scoped` | Update a KDS order's status in the store resolved from a session token. ADR #7. |

## POS and Sales

| `add_line` | Add a line to an active cart using the global database.

**Deprecated for multi-store (ADR #7):** Use `add_line_scoped`. |
| `add_line_scoped` | Add a line to an active cart in the store resolved from a session token. ADR #7.

ADR-19 §5.1: rejects the command when the cart has no `deduction_loc... |
| `apply_promotion` | Apply a promotion to a sale.

**Deprecated for multi-store (ADR #7):** Use `apply_promotion_scoped`. |
| `apply_promotion_scoped` | Apply a promotion in the store resolved from a session token. ADR #7. |
| `complete_sale` | Complete a sale using the global database.

**Deprecated for multi-store (ADR #7):** Use `complete_sale_scoped` |
| `complete_sale_scoped` | Complete a sale within the store resolved from a session token.

ADR #7: Scoped variant of `complete_sale`. The `user_id` for |
| `complete_sale_with_resolved_shortfalls_scoped` | Complete a sale with cashier-resolved shortfalls (split fulfillment).

This is the second command in the two-command flow (ADR-19 §6b). |
| `compute_cart_tax_scoped` | Compute cart tax for the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `create_promotion` | Create a new promotion.

**Deprecated for multi-store (ADR #7):** Use `create_promotion_scoped`. |
| `create_promotion_scoped` | Create a promotion in the store resolved from a session token. ADR #7. |
| `delete_held_cart` | Delete a held cart from the global database.

**Deprecated for multi-store (ADR #7):** Use `delete_held_cart_scoped`. |
| `delete_held_cart_scoped` | Delete a held cart in the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `delete_promotion` | Delete a promotion by id.

**Deprecated for multi-store (ADR #7):** Use `delete_promotion_scoped`. |
| `delete_promotion_scoped` | Delete a promotion in the store resolved from a session token. ADR #7. |
| `freeze_gift_card` | Freeze a gift card.

Prevents further transactions on the card (e.g., for fraud prevention |
| `get_cart_deduction_location` | Return the deduction location info for an active cart.

Returns `null` when the cart has no deduction location lock |
| `get_gift_card` | Get a gift card by its card number or internal ID.

Looks up a gift card and returns it with all associated |
| `get_gift_card_balance` | Get the current balance of a gift card.

Returns the balance in minor units, currency code, and card status. |
| `get_held_cart` | Resume a held cart from the global database.

**Deprecated for multi-store (ADR #7):** Use `get_held_cart_scoped`. |
| `get_held_cart_scoped` | Get a held cart from the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `get_promotion` | Get a single promotion by id.

**Deprecated for multi-store (ADR #7):** Use `get_promotion_scoped`. |
| `get_promotion_scoped` | Get a promotion from the store resolved from a session token. ADR #7. |
| `get_sale_promotions` | List all promotion applications for a sale.

**Deprecated for multi-store (ADR #7):** Use `get_sale_promotions_scoped`. |
| `get_sale_promotions_scoped` | Get sale promotions from the store resolved from a session token. ADR #7. |
| `hold_cart` | Park the current sale as a held order in the global database.

**Deprecated for multi-store (ADR #7):** Use `hold_cart_scoped`. |
| `hold_cart_scoped` | Hold a cart in the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `issue_gift_card` | Issue a new gift card with an initial balance.

Creates a new gift card with a unique card number and stores |
| `list_gift_cards` | List all gift cards with optional filtering by status.

Returns a list of gift cards with their transaction history. |
| `list_held_carts` | List all held carts from the global database.

**Deprecated for multi-store (ADR #7):** Use `list_held_carts_scoped`. |
| `list_held_carts_scoped` | List held carts for the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `list_open_bills` | List open bills from the global database.

**Deprecated for multi-store (ADR #7):** Use `list_open_bills_scoped`. |
| `list_open_bills_scoped` | List open bills for the store resolved from a session token. ADR #7.

Requires `SALES_PROCESS` permission. |
| `list_promotions` | List all promotions.

**Deprecated for multi-store (ADR #7):** Use `list_promotions_scoped`. |
| `list_promotions_scoped` | List promotions for the store resolved from a session token. ADR #7. |
| `list_refunds` | List all refunds for a sale from the global database.

**Deprecated for multi-store (ADR #7):** Use `list_refunds_scoped`. |
| `list_refunds_scoped` | List all refunds for a sale from the store resolved from a session token.

ADR #7: Scoped variant of `list_refunds`. |
| `lookup_sale_by_receipt_barcode` | Look up a sale by its receipt barcode from the global database.

**Deprecated for multi-store (ADR #7):** Use `lookup_sale_by_receipt_barcode_scoped`. |
| `lookup_sale_by_receipt_barcode_scoped` | Look up a sale by receipt barcode from the store resolved from a session token.

ADR #7: Scoped variant of `lookup_sale_by_receipt_barcode`. |
| `override_cart_deduction_location_scoped` | Override the deduction location lock on an active cart.

Records the manager override timestamp (`location_override_at`) on the |
| `override_line_price` | Override the unit price of a cart line using the global database.

**Deprecated for multi-store (ADR #7):** Use `override_line_price_scoped`. |
| `override_line_price_scoped` | Override a line price within the store resolved from a session token.

ADR #7: Scoped variant of `override_line_price`. The `user_id` for |
| `process_refund` | Process a refund against a completed sale using the global database.

**Deprecated for multi-store (ADR #7):** Use `process_refund_scoped` |
| `process_refund_scoped` | Process a refund within the store resolved from a session token.

ADR #7: Scoped variant of `process_refund`. The `user_id` for |
| `redeem_gift_card` | Redeem (spend) a gift card balance against a sale.

Deducts the specified amount from the card balance and records |
| `set_cart_discount` | Set or clear a cart-level percentage discount using the global database.

**Deprecated for multi-store (ADR #7):** Use `set_cart_discount_scoped` |
| `set_cart_discount_scoped` | Set a cart discount within the store resolved from a session token.

ADR #7: Scoped variant of `set_cart_discount`. The `user_id` for |
| `start_sale` | Start a new sale cart using the global database.

**Deprecated for multi-store (ADR #7):** Use `start_sale_scoped` |
| `start_sale_scoped` | Start a new sale in the store resolved from a session token. ADR #7.

ADR-19 §5.1: resolves the primary deduction location from the workspace |
| `top_up_gift_card` | Add value (top up) to an existing gift card.

Increases the card's balance by the specified amount and records |
| `unfreeze_gift_card` | Unfreeze a previously frozen gift card.

Restores normal transaction capabilities to the card. |
| `update_promotion` | Update an existing promotion.

**Deprecated for multi-store (ADR #7):** Use `update_promotion_scoped`. |
| `update_promotion_scoped` | Update a promotion in the store resolved from a session token. ADR #7. |
| `void_sale` | Void an active (completed) sale using the global database.

**Deprecated for multi-store (ADR #7):** Use `void_sale_scoped` |
| `void_sale_scoped` | Void a sale within the store resolved from a session token.

ADR #7: Scoped variant of `void_sale`. The `user_id` for permission |

## Products and Inventory

| `adjust_stock` | No description available. |
| `adjust_stock_scoped` | Adjust stock for the store resolved from a session token.

ADR #7: Scoped variant of `adjust_stock`. The frontend passes a |
| `create_category` | Create category.

**Deprecated for multi-store (ADR #7):** Use `create_category_scoped`, |
| `create_category_scoped` | Create category in the store resolved from a session token (CAT-01).

Resolves the store from the opaque session token and enforces |
| `create_product` | Create a product using the global database and a `user_id` parameter.

**Deprecated for multi-store (ADR #7):** Use `create_product_scoped` |
| `create_product_scoped` | Create a product within the store resolved from a session token.

ADR #7: Scoped variant of `create_product`. The `user_id` for |
| `create_product_variant` | Create a new product variant. |
| `delete_category` | Delete category.

**Deprecated for multi-store (ADR #7):** Use `delete_category_scoped`. |
| `delete_category_scoped` | Delete a category in the store resolved from a session token (CAT-01/02).

Enforces `products:delete` on the session user, then deletes the |
| `delete_product` | Delete a product using the global database and a `user_id` parameter.

**Deprecated for multi-store (ADR #7):** Use `delete_product_scoped` |
| `delete_product_scoped` | Delete a product within the store resolved from a session token.

ADR #7: Scoped variant of `delete_product`. The `user_id` for |
| `delete_product_variant` | Delete a product variant by its own SKU. |
| `get_product_track_serial` | Check whether a product tracks serial numbers.

**Deprecated for multi-store (ADR #7):** Use `get_product_track_serial_scoped`. |
| `get_product_track_serial_batch` | Check serial-tracking flags for many SKUs in one round trip
(PERF-03: replaces the N+1 `get_product_track_serial` loop).
 |
| `get_product_track_serial_batch_scoped` | Store-scoped batch variant of `get_product_track_serial_batch`. ADR #7. |
| `get_product_track_serial_scoped` | Check whether a product tracks serial numbers, store-scoped. ADR #7. |
| `get_product_variant` | Get a single variant by its own SKU. |
| `list_categories` | Fetch all categories, ordered by name.

**Deprecated for multi-store (ADR #7):** Use `list_categories_scoped`. |
| `list_categories_scoped` | Fetch all categories for the store resolved from a session token. ADR #7. |
| `list_product_variants` | List all variants for a given parent product SKU. |
| `list_products` | Fetch all products from the database.

Returns an array of product DTOs with category names and stock |
| `list_products_scoped` | Fetch all products for the store resolved from a session token.

ADR #4 / ADR #7 canonical pattern: The frontend passes an opaque |
| `lookup_by_barcode` | Look up a single product by barcode.

Returns the product DTO or `null` when no match is found. |
| `lookup_by_barcode_scoped` | Look up a product by barcode for the store resolved from a
session token. ADR #7 scoped variant. |
| `lookup_product_by_sku` | Look up a single product by SKU.

Returns the product DTO or `null` when no match is found. |
| `lookup_product_by_sku_scoped` | Look up a product by SKU for the store resolved from a
session token. ADR #7 scoped variant. |
| `update_category` | Update an existing category's name, colour, and icon.

**Deprecated for multi-store (ADR #7):** Use `update_category_scoped`. |
| `update_category_scoped` | Update a category in the store resolved from a session token (CAT-01).

Enforces `products:update` on the session user. ADR #7. |
| `update_product` | Update a product using the global database and a `user_id` parameter.

**Deprecated for multi-store (ADR #7):** Use `update_product_scoped` |
| `update_product_scoped` | Update a product within the store resolved from a session token.

ADR #7: Scoped variant of `update_product`. The `user_id` for |
| `update_product_variant` | Update an existing product variant (matched by SKU). |

## Reporting and Analytics

| `build_custom_report` | Build a custom report from user-selected columns and filters.

**Deprecated for multi-store (ADR #7):** Use `build_custom_report_scoped`. |
| `build_custom_report_scoped` | Build a custom report for the session's store.

Custom reports can expose customer and staff data, so exporting them |
| `export_daily_summary` | Fetch the daily sales summary from the global database.

**Deprecated for multi-store (ADR #7):** Use `export_daily_summary_scoped` |
| `export_daily_summary_scoped` | Fetch the daily sales summary for the store resolved from a session token.

ADR #7: Scoped variant of `export_daily_summary`. |
| `export_eod_report` | Fetch the full EOD (End-of-Day) report from the global database.

**Deprecated for multi-store (ADR #7):** Use `export_eod_report_scoped`. |
| `export_eod_report_scoped` | Fetch the full EOD report for the store resolved from a session token.

ADR #7: Scoped variant of `export_eod_report`. Opens the store-scoped |
| `export_sales_by_hour` | Fetch sales-by-hour breakdown from the global database.

**Deprecated for multi-store (ADR #7):** Use `export_sales_by_hour_scoped`. |
| `export_sales_by_hour_scoped` | Fetch sales-by-hour breakdown for the store resolved from a session token.

ADR #7: Scoped variant of `export_sales_by_hour`. |
| `get_sale` | Fetch a single sale by ID from the global database.

**Deprecated for multi-store (ADR #7):** Use `get_sale_scoped` |
| `get_sale_scoped` | Fetch a single sale by ID from the store resolved from a session token.

ADR #7: Scoped variant of `get_sale`. The backend resolves the |
| `list_sales` | List all sales from the global database.

**Deprecated for multi-store (ADR #7):** Use `list_sales_scoped` |
| `list_sales_scoped` | List all sales for the store resolved from a session token.

ADR #7: Scoped variant of `list_sales`. The backend resolves the |

## Settings and Configuration

| `create_store_profile` | Create a new store profile (non-primary by default).

ADR #4 Phase 2: Also creates the per-store SQLite database file |
| `create_tax_rate_scoped` | Create a tax rate in the store resolved from a session token. ADR #7.

TAX-01: resolves the store from the session, enforces `SETTINGS_EDIT` |
| `delete_store_profile` | Delete a non-primary store profile. |
| `delete_tax_rate_scoped` | Delete a tax rate in the store resolved from a session token. ADR #7.

TAX-01: resolves the store from the session and enforces |
| `get_primary_store` | Get the primary store profile. |
| `get_receipt_settings_scoped` | Get receipt settings resolved from a session token. ADR #7. |
| `get_setting` | Read a single setting value by key.

Returns `None` when the key does not exist. |
| `get_store_profile` | Get a single store profile by id. |
| `get_store_settings_scoped` | Get store settings resolved from a session token. ADR #7. |
| `get_tax_rate_dependency_counts_scoped` | Get dependency (reference) counts for a tax rate in the store resolved
from a session token. ADR #7.
 |
| `get_user_preferences` | **Deprecated — use `get_user_preferences_scoped` (ADR #7).** |
| `get_user_preferences_scoped` | Get user preferences resolved from a session token. ADR #7.
Uses `session.user_id` for the preference lookup. |
| `list_category_tax_rates_scoped` | List category-to-tax-rate assignments for the store resolved from a
session token. ADR #7. TAX-01: session-scoped with `SETTINGS_READ`. |
| `list_credit_sales` | List credit sales.

**Deprecated for multi-store (ADR #7):** Use `list_credit_sales_scoped`. |
| `list_credit_sales_scoped` | List credit sales for the store resolved from a session token. ADR #7. |
| `list_store_profiles` | List all store profiles. |
| `list_tax_rates_scoped` | List tax rates for the store resolved from a session token. ADR #7.

TAX-01: session-scoped read with `SETTINGS_READ` on the backend. |
| `set_category_tax_rates_scoped` | Set (replace) the tax rates assigned to a category in the store resolved
from a session token. ADR #7. TAX-01: session-scoped with `SETTINGS_EDIT`. |
| `set_credit_settings` | **Deprecated — use `set_credit_settings_scoped` (ADR #7).** |
| `set_credit_settings_scoped` | Set credit settings resolved from a session token. ADR #7. |
| `set_hardware_settings` | **Deprecated — use `set_hardware_settings_scoped` (ADR #7).**

Writes to both DB (canonical) and JSON file (fallback). |
| `set_hardware_settings_scoped` | Set hardware settings resolved from a session token. ADR #7.

Writes to both DB (canonical) and JSON file (fallback). |
| `set_primary_store` | Promote a store to primary (demoting the current primary). |
| `set_receipt_settings` | **Deprecated — use `set_receipt_settings_scoped` (ADR #7).** |
| `set_receipt_settings_scoped` | Set receipt settings resolved from a session token. ADR #7. |
| `set_setting` | **Deprecated — use `set_setting_scoped` (ADR #7).**

Write (or overwrite) a single setting value. |
| `set_setting_scoped` | Write (or overwrite) a single setting value resolved from a session token. ADR #7.

Pass an empty string to store an empty value. |
| `set_settings` | Write (or overwrite) multiple settings in a single transaction.

All entries are written atomically — either all succeed or none |
| `set_settings_scoped` | Write (or overwrite) multiple settings in a single transaction, resolved from a session token. ADR #7.

All entries are written atomically — either al... |
| `set_store_settings` | **Deprecated — use `set_store_settings_scoped` (ADR #7).** |
| `set_store_settings_scoped` | Set store settings resolved from a session token. ADR #7. |
| `set_user_preferences` | **Deprecated — use `set_user_preferences_scoped` (ADR #7).** |
| `set_user_preferences_scoped` | Set user preferences resolved from a session token. ADR #7.
Uses `session.user_id` for the preference write. |
| `settle_credit` | **Deprecated — use `settle_credit_scoped` (ADR #7).** |
| `settle_credit_scoped` | Settle a credit sale resolved from a session token. ADR #7. |
| `update_store_profile` | Update a store profile's mutable fields. |
| `update_tax_rate_scoped` | Update a tax rate in the store resolved from a session token. ADR #7.

TAX-01: resolves the store from the session and enforces |

## Shifts and Staff

| `bootstrap_owner` | Create the first owner user in a fresh installation.

This is the only command that does NOT require an existing session, |
| `close_shift` | Close an active shift using the global database.

**Deprecated for multi-store (ADR #7):** Use `close_shift_scoped`. |
| `close_shift_scoped` | Close a shift in the store resolved from a session token. ADR #7. |
| `create_cash_payout` | Record a cash payout (safe drop) against an open shift. |
| `create_staff_scoped` | Create a staff member. Caller identity is resolved from the session token.

STAFF-02: enforces the role-assignment hierarchy (only Owner-level |
| `get_active_shift` | Get the currently open shift for a user from the global database.

**Deprecated for multi-store (ADR #7):** Use `get_active_shift_scoped`. |
| `get_active_shift_scoped` | Get the active shift for the session user from the store-scoped DB. ADR #7. |
| `get_shift` | Get a single shift by id. |
| `get_shift_report` | Generate a comprehensive report for a single shift. |
| `list_roles_scoped` | List roles. Caller identity is resolved from the session token. |
| `list_shifts` | List all shifts, most recent first.

**Deprecated for multi-store (ADR #7):** Use `list_shifts_scoped`. |
| `list_shifts_scoped` | List shifts for the store resolved from a session token. ADR #7. |
| `list_staff` | List staff.

**Deprecated for multi-store (ADR #7):** Use [`list_staff_scoped`] so the |
| `list_staff_scoped` | List staff members. Caller identity is resolved from the session token. |
| `open_shift` | Open a new shift for a user using the global database.

**Deprecated for multi-store (ADR #7):** Use `open_shift_scoped`. |
| `open_shift_scoped` | Open a shift in the store resolved from a session token. ADR #7. |
| `update_staff_scoped` | Update a staff member. Caller identity is resolved from the session token.

STAFF-02: enforces the role-assignment hierarchy (Owner-only promotion, |

## Stock Management

| `acknowledge_stock_alert_scoped` | Acknowledge a stock alert event (records who acknowledged it).

Requires `SALES_PROCESS` permission. |
| `active_stock_alerts_scoped` | Get active stock alerts for a location (enriched with product SKU/name).

Requires `SALES_PROCESS` permission. |
| `add_count_line_scoped` | Add a line to an editable count in the session's store. |
| `add_stock_transfer_line_scoped` | Add a transfer line in the session-scoped store. |
| `cancel_stock_transfer_scoped` | Cancel a transfer in the session-scoped store. |
| `complete_stock_count_scoped` | Complete a count and attribute generated adjustments to the session user. |
| `create_inventory_location` | Create a new inventory location.

Requires `INVENTORY_LOCATIONS_MANAGE` permission (LOC-06) — location |
| `create_inventory_transaction` | Create a new manual / staff inventory transaction audit log session.

Requires `SALES_PROCESS` permission. |
| `create_stock_count_scoped` | Create a stock count in the session's store and attribute it to the session user. |
| `deactivate_inventory_location` | Deactivate an inventory location (fails if contains stock or pending transfers).

Requires `INVENTORY_LOCATIONS_MANAGE` permission (LOC-06). |
| `delete_stock_threshold` | Delete a stock alert threshold boundary.

Requires `SALES_PROCESS` permission. |
| `end_inventory_shift` | End an active inventory shift.

Requires `SALES_PROCESS` permission. |
| `finalize_sale` | Transition a pending sale's status to completed after payment capture.

Requires `SALES_PROCESS` permission. |
| `get_active_inventory_shift` | Retrieve the active inventory shift for the current user, if any.

Requires `SALES_PROCESS` permission. |
| `get_count_lines_scoped` | Fetch lines from a count in the session's store. |
| `get_inventory_transaction` | Retrieve details of a single transaction, including its lines.

Requires `SALES_PROCESS` permission. |
| `get_low_stock_alerts_at_location_scoped` | Get per-location low stock alerts.

Requires `SALES_PROCESS` permission. |
| `get_stock_count_scoped` | Fetch one stock count from the session's store. |
| `get_stock_thresholds` | Get stock alert thresholds for a location.

Requires `SALES_PROCESS` permission. |
| `get_stock_transfer_lines_scoped` | Get transfer lines from the session-scoped store. |
| `get_stock_transfer_scoped` | Get a stock transfer from the session-scoped store. |
| `get_workspace_inventory_locations` | Get inventory location bindings for a workspace instance.

Requires `INVENTORY_VIEW` permission (LOC-06). |
| `get_workspace_locations_scoped` | Resolve locations bound to a workspace instance (unified resolver ADR-19 §10).

Requires `INVENTORY_VIEW` permission (LOC-06) — reading the bound-loca... |
| `invalidate_location_cache_scoped` | Invalidate the location resolver cache.

Requires `INVENTORY_VIEW` permission (LOC-06) — cache invalidation is a |
| `list_in_transit_transfers_scoped` | List in-transit transfers with their line items in one batch request.

The transit audit screen previously listed all transfers and then fetched |
| `list_inventory_locations` | List all inventory locations.

Requires `INVENTORY_VIEW` permission (LOC-06) — reading the picker list |
| `list_inventory_shifts` | List all inventory shifts history.

Requires `SALES_PROCESS` permission. |
| `list_inventory_transactions` | List all inventory transactions.

Requires `SALES_PROCESS` permission. |
| `list_inventory_transactions_for_shift` | List inventory transactions for a specific shift (staff + location + time window).

Used by the inventory shift-bar summary to avoid client-side filte... |
| `list_stock_adjustments_scoped` | List adjustments from the session's store. |
| `list_stock_counts_scoped` | List stock counts from the session's store. |
| `list_stock_transfers_scoped` | List stock transfers from the session-scoped store. |
| `receive_stock_transfer_scoped` | Receive a transfer, attributing the actor to the authenticated session. |
| `remove_count_line_scoped` | Remove a line belonging to an editable count in the session's store. |
| `remove_stock_transfer_line_scoped` | Remove a transfer line in the session-scoped store. |
| `send_stock_transfer_scoped` | Send a transfer in the session-scoped store. |
| `set_stock_threshold` | Set a stock alert threshold boundary.

Requires `SALES_PROCESS` permission. |
| `set_workspace_inventory_locations` | Set inventory location bindings for a workspace instance.

Requires `INVENTORY_LOCATIONS_MANAGE` permission (LOC-06) — binding is a |
| `start_inventory_shift` | Start a new inventory shift for the current user at a location.

Requires `SALES_PROCESS` permission. |
| `update_count_line_scoped` | Update a line belonging to an editable count in the session's store. |
| `update_inventory_location` | Update details of an existing inventory location.

Requires `INVENTORY_LOCATIONS_MANAGE` permission (LOC-06). |
| `update_stock_count_status_scoped` | Move an editable count to `in_progress` or `cancelled`. |
| `void_pending_sale` | Void a pending sale and restore stock.

Requires `SALES_PROCESS` permission. |

## Sync and Offline

| `delete_offline_item` | Delete a processed offline queue item. |
| `enqueue_offline` | Manually enqueue a transaction for later sync. |
| `get_sync_settings` | Get sync settings. |
| `get_sync_settings_scoped` | Get sync settings resolved from a session token. ADR #7. |
| `list_all_offline` | List all offline queue items (most recent first). |
| `list_pending_offline` | List all pending (unsynced) offline queue items, oldest first. |
| `list_remote_failures` | List retained remote-application failures (dead-letter discovery).

Operators call this to discover which remote items are quarantined (or |
| `offline_queue_status_summary` | Get a summary of the offline queue status (P1-6 sync observability).
Returns counts by status, conflict count, and timing info. |
| `pending_offline_count` | Get the count of pending offline items. |
| `pending_sync_count` | Get the pending sync count. |
| `request_sync_token` | Request a new JWT API token from the cloud server's
`POST /api/v1/tokens` endpoint. When the server is configured with an
`OZ_ADMIN_KEY` (production), the mint requires the matching `X-Admin-Key`
header (ADR sync-auth-hardening P2).
 |
| `requeue_remote_failure` | Requeue a dead-lettered remote item so the next sync cycle retries it.

Operators call this after remediating the item's source (e.g. creating |
| `retry_offline_sync` | Attempt to sync all pending offline items through the real cloud sync
pipeline. Returns a `SyncAttemptResult`; when the tenant is on the `free`
plan (server has `OZ_ENFORCE_PLANS=1`) the result carries `plan_required:
true` and items stay `pending` — they are never marked failed or
quarantined (ADR sync-plan-gating).
 |
| `sync_pull` | Pull a server snapshot and overwrite the local cache for products,
tax rates, and users.
 |
| `sync_run` | Immediately run a sync cycle that pushes pending sales, credit, and
other queued offline transactions to the configured cloud server. Returns a
`SyncAttemptResult`; a `403 plan_required` from the server (free tenant,
`OZ_ENFORCE_PLANS=1`) sets `plan_required: true` instead of marking items
failed — queued items stay pending and sync automatically after an upgrade
(ADR sync-plan-gating).
 |
| `test_sync_connection` | Test the cloud sync connection by pinging the configured server's
`/health` endpoint.
 |

## Utilities and Misc

| `activate_license` | Activates a license key for the given email, phone, and machine ID. |
| `check_license_status` | Checks the license status against the PocketBase license server.

Unlike [`get_license_status`] which reads locally-stored data, this |
| `complete_setup` | Persist the chosen preset and features, then mark setup as complete.

Called by the front-end when the user clicks "Complete Setup" on |
| `create_bundle` | Create a new bundle. |
| `create_customer_scoped` | Create a customer in the store resolved from a session token.

The session supplies both the store database and authenticated user; |
| `delete_bundle` | Delete a bundle. |
| `delete_customer_scoped` | Delete a customer from the store resolved from a session token. ADR #7. |
| `dismiss_setup_wizard` | Dismiss the setup wizard without enabling any features.

Called when the user clicks "Skip setup". Only writes the |
| `earn_loyalty_points_scoped` | Awards loyalty points in the store resolved by the active session. |
| `export_audit_log_scoped` | Export the session store's audit log to CSV (AUD-09).

Resolves the store and authenticated user from the session token, |
| `get_audit_review_status_scoped` | Fetch the session store's latest review checkpoint + unreviewed count
(AUD-04). Resolves the store from the session token and enforces
`audit:view`. |
| `get_brand_settings` | Load all brand settings at once. |
| `get_brand_settings_scoped` | Load all brand settings resolved from a session token. ADR #7. |
| `get_bundle` | Get a single bundle by id. |
| `get_customer_history_scoped` | Get the read-only history for a customer (CUST-05). ADR #7.

Scoped to the session's store and gated on `customers:view`. Sales are |
| `get_device_id` | Get the stable device identifier (hostname) for terminal binding.

Reads `COMPUTERNAME` on Windows, `HOSTNAME` on Unix, or falls back |
| `get_enabled_features` | Return the list of currently-enabled feature keys.

The front-end calls this once on mount to decide which nav items |
| `get_key_rotation_info` | Get the current key rotation status (key age, creation timestamp).

Returns the status without exposing the key material itself. |
| `get_license_status` | Analyzes the local license state and returns a comprehensive status response. |
| `get_local_ip` | Get the local IP address of the machine. |
| `get_loyalty_account_scoped` | Retrieves a loyalty account from the store resolved by the active session. |
| `get_machine_id` | Retrieves the unique hardware identifier for this installation. |
| `get_or_create_loyalty_account_scoped` | Retrieves or creates a loyalty account in the active store. |
| `get_points_value_scoped` | Converts loyalty points into minor currency units in the active store. |
| `get_report_schedule` | Get the current report schedule configuration.

Returns the saved [`ReportScheduleConfig`] or a default if none |
| `get_setup_status` | Returns whether the setup wizard has been completed.

The front-end calls this on mount to decide whether to render |
| `list_all_features` | Fetch every known feature with its current enabled status, metadata,
and dependency information.
 |
| `list_audit_log` | Fetch audit log entries in reverse chronological order.

Supports pagination via `limit` and `offset`. Returns an array of |
| `list_audit_log_scoped` | Fetch audit log entries scoped to the session's store (AUD-01).

Resolves the store and authenticated user from the session token, |
| `list_bundles` | List all bundles with their items. |
| `list_customers_scoped` | List customers for the store resolved from a session token. ADR #7. |
| `list_loyalty_accounts_scoped` | Lists loyalty accounts from the store resolved by the active session. |
| `list_loyalty_tiers_scoped` | Lists loyalty tiers from the store resolved by the active session. |
| `list_scale_devices` | List all registered weight scales. |
| `lookup_bundle_by_sku` | Look up a bundle by its SKU (for barcode scanning / POS lookup). |
| `mark_audit_reviewed_scoped` | Persist a server-side review checkpoint for the session's store (AUD-04).

Writes the checkpoint row and an `audit.review` audit event in one |
| `pick_logo_file` | Open a native file picker filtered to image files and return the
chosen path, or `None` if the user cancelled. |
| `ping` | Liveness probe. Returns `Ok("pong")` if the Tauri runtime is alive. |
| `read_scale_weight` | Read the current weight from the registered weight scale.

Uses the default scale registered under the "default" key. |
| `redeem_loyalty_points_scoped` | Redeems loyalty points in the store resolved by the active session. |
| `renew_license` | Renews an existing license subscription with a new license key.

Calls the server's `/api/v1/license/renew` endpoint with the |
| `rotate_encryption_key` | Rotate (re-generate) the encryption key.

Generates a new random 256-bit AES key, archives the previous key, |
| `save_report_schedule` | Save the report schedule configuration. |
| `search_customers_scoped` | Search customers in the store resolved from a session token. ADR #7.

CUST-06: the query runs server-side (LIKE over name/email/phone) with a |
| `seed_default_roles_scoped` | Requires the `staff:manage_roles` permission. |
| `send_test_report` | Send a test report email using the currently configured SMTP
settings and report schedule.
 |
| `set_brand_logo_path` | Set the filesystem path to the store logo.

The path is validated to ensure it: |
| `set_brand_primary_colour` | Set the primary brand colour. |
| `set_brand_store_name` | Set the brand store display name. |
| `set_feature` | Enable or disable a single feature flag.

When enabling, all required dependencies are automatically enabled |
| `set_features_bulk` | Enable or disable multiple feature flags atomically in a single
SQLite transaction.
 |
| `update_bundle` | Update an existing bundle. |
| `update_customer_scoped` | Update a customer in the store resolved from a session token. ADR #7. |
| `update_loyalty_tier_scoped` | Updates a loyalty tier in the store resolved by the active session. |
| `version_scoped` | Version info resolved from a session token. ADR #7.
Validates the session token and returns the same compile-time version info. |

## Workspaces and Multi-Store

| `apply_topology_diff` | Apply a full topology diff atomically (Critical #4).

Creates, updates, and archives workspace instances within a single |
| `archive_workspace_instance_scoped` | Archive (soft-delete) a workspace instance (admin). ADR #7.

Sets the instance status to `archived`, preserving referential |
| `create_workspace_instance_scoped` | Create a new workspace instance (admin). Permission from session. ADR #7.

ADR #5: Enforces subscription tier quota before creating. |
| `get_user_workspace_instances_scoped` | Get instance IDs assigned to a user. Permission check from session. ADR #7. |
| `get_user_workspaces` | Get the explicit workspace keys assigned to a user (legacy table).

**Deprecated for multi-store (ADR #7):** Use `get_user_workspace_instances_scoped`... |
| `get_user_workspaces_scoped` | Get workspace keys for a user (legacy table), caller from session. ADR #7. |
| `get_workspace_instance_scoped` | Get a single workspace instance. `is_default` reflects the session user. ADR #7. |
| `list_all_workspaces` | List ALL workspace types (for admin dropdowns).

**Deprecated for multi-store (ADR #7):** Use `list_workspaces_scoped` instead. |
| `list_all_workspaces_scoped` | List all workspace types resolved from a session token. ADR #7. |
| `list_workspace_screens` | List screens (nav items) for a workspace type during boot/workspace
selection. The store ID is explicit so the read is routed to the correct
store dat... |
| `list_workspace_screens_scoped` | List screens for a workspace type from the store-scoped database. ADR #7. |
| `list_workspaces` | List workspace instances for the pre-session workspace picker.

This narrow bootstrap command runs after username/PIN authentication but |
| `list_workspaces_for_store_scoped` | List workspace instances in an explicitly named store for the session user.

Authenticated replacement for the terminal-management screen's use of the |
| `list_workspaces_scoped` | List workspace instances accessible to the session user within their store. ADR #7.

ADR #5: Filters results by subscription tier entitlement. |
| `load_topology` | Load the persisted topology graph.

Returns `None` when no topology has been saved yet (the front-end |
| `recover_workspace_instances_scoped` | Recover `QuotaSuspended` workspace instances after a tier upgrade. ADR #5 Phase 3b.

Iterates the target store's database, restores suspended instance... |
| `resolve_boot_store` | Resolve the active store and instance from device binding.

This is called once at boot time (before authentication). It does not use |
| `set_user_workspace_instances_scoped` | Replace all instance assignments for a user. Caller permission from session. ADR #7. |
| `set_user_workspaces` | Replace all workspace assignments for a user (legacy tables).

**Deprecated for multi-store (ADR #7):** Use `set_user_workspace_instances_scoped`. |
| `set_user_workspaces_scoped` | Replace all workspace assignments for a user (legacy tables), caller from session. ADR #7. |
| `suspend_surplus_workspace_instances_scoped` | Suspend surplus workspace instances after a tier downgrade. ADR #5 Phase 3c.

If the store has more active instances than the tier allows, the |
| `update_workspace_instance_scoped` | Update the editable fields of a workspace instance (admin). ADR #7.

Renames the instance and updates its description / accent colour. |

> last audited 09-08-26 by buffy
> audit: Phase 1 Core Architecture & API Docs Audit

> status: ACCURATE (verified against actual codebase) · verified accurate: all 365 commands extracted from #[tauri::command] macros, organized into 12 feature categories

