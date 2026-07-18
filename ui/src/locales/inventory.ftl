# ui/src/locales/inventory.ftl — Inventory adjustment

inv-title = Inventory Adjustment
inv-step-select-product = 1. Select Product
inv-step-adjustment-details = 2. Adjustment Details
inv-change = Change
inv-change-aria = Change product
inv-search-placeholder = Search by SKU, name, or barcode…
inv-search-aria = Search products
inv-loading = Loading products…
inv-no-results = No products match your search.
inv-hint = Type to search for a product by SKU, name, or barcode.
inv-stock-count = { $count } in stock
inv-stock-off = Stock tracking off
inv-type-aria = Adjustment type
inv-type-add-aria = Stock In
inv-type-add-label = Stock In (Restock)
inv-type-remove-aria = Stock Out
inv-type-remove-label = Stock Out (Remove)
inv-qty-label = Quantity
inv-qty-placeholder = e.g. 10
inv-qty-hint = Current stock: { $stock }
inv-reason-label = Reason
inv-reason-select = Select a reason…
inv-reason-custom-label = Describe the reason
inv-reason-custom-placeholder = Enter the reason for this adjustment…
inv-error = { $message }
inv-success-adjusted = Adjusted &quot;{ $name }&quot; by { $delta }. New stock: { $newQty }
inv-error-qty-positive = Quantity must be a positive number
inv-error-reason-required = Please select or enter a reason
inv-error-stock-insufficient = Cannot remove { $qty } units — only { $stock } in stock
inv-error-generic = Failed to adjust stock
inv-cancel = Cancel
inv-apply-restock = Apply Restock
inv-apply-removal = Apply Removal
inv-adjusting = Adjusting…
inv-reason-restock = Restock (supplier delivery)
inv-reason-stock-take = Stock take correction
inv-reason-return = Customer return
inv-reason-damaged = Damaged / spoiled
inv-reason-write-off = Write-off / expiry
inv-reason-transfer = Transfer to other location
inv-reason-other = Other reason…
inv-report-title = Inventory Report
inv-report-threshold = Threshold
inv-report-export-csv = Export CSV
inv-report-sku = SKU
inv-report-product = Product
inv-report-current-stock = Stock
inv-report-loading-aria = Loading inventory report
inv-report-region-aria = Inventory Report
inv-report-threshold-aria = Stock threshold
inv-report-print-aria = Print report
inv-report-export-aria = Export CSV
inv-report-csv-header-sku = SKU
inv-report-csv-header-product = Product
inv-report-csv-header-stock = Current Stock
inv-report-csv-header-threshold = Threshold
inv-report-no-results = No results found
inv-search-results-aria = Search results
inv-qty-field-aria = Quantity
inv-reason-custom-field-aria = Describe the reason

# Inventory Shifts
inv-shift-start-title = Start Inventory Shift
inv-shift-select-location = Select Location
inv-shift-notes-label = Shift Notes
inv-shift-notes-placeholder = e.g., Night shift count...
inv-shift-start-btn = Start Shift
inv-shift-active-info = { $user } — { $location } — Started { $time }
inv-shift-end-btn = End Shift
inv-shift-summary-title = Shift Summary
inv-shift-summary-performed = Transactions performed during this shift:
inv-shift-no-transactions = No transactions recorded.

# Transit Audit
inv-transit-title = Transit Stock Audit
inv-transit-col-sku = SKU
inv-transit-col-product = Product
inv-transit-col-qty = Qty
inv-transit-col-source = Source
inv-transit-col-dest = Destination
inv-transit-col-sent = Sent At
inv-transit-col-overdue = Overdue
inv-transit-reverse-btn = Reverse Transfer
inv-transit-no-overdue = No overdue transit items.

# Transaction Log
inv-log-title = Inventory Transaction Log
inv-log-filter-location = Location
inv-log-filter-staff = Staff
inv-log-filter-type = Type
inv-log-filter-all = All
inv-log-expand-btn = Details
inv-log-col-barcode = Barcode Scanned

# Threshold Config
inv-threshold-title = Stock Threshold Configuration
inv-threshold-col-sku = SKU
inv-threshold-col-product = Product Name
inv-threshold-col-location = Location
inv-threshold-col-threshold = Threshold
inv-threshold-add-btn = + Add Threshold
inv-threshold-dialog-title = Configure Threshold
inv-threshold-global-opt = Global (All Locations)

# Stock Alert Panel
inv-alert-title = Stock Alert Panel
inv-alert-badge-count = { $count } Stock Alerts
inv-alert-col-triggered = Triggered
inv-alert-acknowledge-btn = Acknowledge

