# ui/src/locales/sales.ftl — POS, cart, sales history, dashboard, refunds

# Cart
cart-title = [TH] Cart [/TH]
cart-empty = [TH] Cart is empty [/TH]
cart-line-remove = [TH] Remove [/TH]
cart-total-label = [TH] Total [/TH]
cart-line-add-sample = [TH] Add sample line [/TH]
cart-line-add-sample-aria = [TH] Add a sample product to the cart for testing [/TH]

# POS
sale-pay-button = [TH] Pay [/TH]
sale-pay-button-aria = [TH] Charge the customer for the current cart [/TH]
pos-title = [TH] POS Terminal [/TH]
pos-cart-panel-title = [TH] Current Sale [/TH]
pos-cart-deducting-label = [TH] Deducting: { $name } [/TH]
pos-cart-deduction-badge-aria = [TH] Deducting from { $name } [/TH]
pos-cart-unbound-error = [TH] Cart has no deduction location — cannot add items [/TH]
pos-cart-empty = [TH] Cart is empty [/TH]
pos-cart-empty-subtitle = [TH] Tap a menu item to start the order [/TH]
pos-cart-total = [TH] Total [/TH]
pos-cart-qty-label = [TH] Qty [/TH]
pos-cart-remove = [TH] Remove [/TH]
pos-cart-pay = [TH] Charge [/TH]
pos-login-required-title = [TH] Login Required [/TH]
pos-login-required-message = [TH] Please log in to use the POS. [/TH]

# Bundle Expansion
pos-bundle-expanded =
    { $count ->
        [one] Bundle "{ $name }" added — 1 item to cart
       *[other] Bundle "{ $name }" added — { $count } items to cart
    }
pos-no-barcode-match = [TH] No product or bundle matches this barcode [/TH]
pos-close-shift-cart-error = [TH] Complete or clear the current sale before closing the shift. [/TH]
pos-close-shift-failed = [TH] Failed to close shift [/TH]

# Scanner
pos-scanner-error = [TH] Scanner error: { $detail } [/TH]

# Payment Modal
payment-dialog-aria =
    .aria-label = [TH] Payment [/TH]
payment-title = [TH] Complete Order [/TH]
payment-table-number =
    .aria-label = [TH] Table number [/TH]
    Table { $number }
payment-close-aria =
    .aria-label = [TH] Cancel payment [/TH]
payment-done-title = [TH] Sale Complete [/TH]
payment-change-label = [TH] Change due [/TH]
payment-done-receipt = [TH] Receipt printed [/TH]
payment-total-due = [TH] Total Due [/TH]
payment-currency-aria =
    .aria-label = [TH] Charge currency [/TH]
payment-currency-label = [TH] Charge Currency [/TH]
payment-currency-select-aria =
    .aria-label = [TH] Select charge currency [/TH]
payment-exchange-aria =
    .aria-label = [TH] Exchange rate information [/TH]
payment-exchange-rate = [TH] Exchange rate [/TH]
payment-rate-source = [TH] Rate source [/TH]
payment-rate-timestamp = [TH] Rate timestamp [/TH]
payment-rate-source-manual = [TH] manual [/TH]
payment-receipt-currency-aria =
    .aria-label = [TH] Receipt currency information [/TH]
payment-charged-in = [TH] Charged in [/TH]
payment-default-currency = [TH] Default currency [/TH]
payment-base-amount = [TH] Base amount [/TH]
payment-charge-amount = [TH] Charge amount [/TH]
payment-method-label = [TH] Payment Method [/TH]
payment-method-cash = [TH] Cash [/TH]
payment-method-card = [TH] Card [/TH]
payment-method-qris = [TH] QRIS [/TH]
payment-other-placeholder =
    .placeholder = [TH] Other... [/TH]
payment-other-aria =
    .aria-label = [TH] Other payment method name [/TH]
payment-amount-tendered = [TH] Amount Tendered [/TH]
payment-tendered-input =
    .placeholder = [TH] 0.00 [/TH]
    .aria-label = [TH] Amount tendered [/TH]
payment-quick-tender-aria =
    .aria-label = [TH] Tender { $amount } [/TH]
payment-tender-exact-aria =
    .aria-label = [TH] Tend exact amount [/TH]
payment-tender-exact = [TH] Exact [/TH]
payment-customer-name-aria =
    .aria-label = [TH] Customer name for open bill [/TH]
payment-change = [TH] Change [/TH]
payment-insufficient = [TH] Insufficient amount [/TH]
payment-qris-description = [TH] Generate a QRIS QR code for the customer to scan with their payment app. [/TH]
payment-qris-btn-aria =
    .aria-label = [TH] Generate QRIS QR code [/TH]
payment-qris-pay = [TH] Pay with QR [/TH]
payment-split-title = [TH] Split Payments [/TH]
payment-split-evenly-aria =
    .aria-label = [TH] Split evenly [/TH]
payment-split-evenly = [TH] Split Evenly [/TH]
payment-split-add-aria =
    .aria-label = [TH] Add split [/TH]
payment-split-add = [TH] + Add Split [/TH]
payment-split-method-cash = [TH] Cash [/TH]
payment-split-method-card = [TH] Card [/TH]
payment-split-other-placeholder =
    .placeholder = [TH] Other [/TH]
payment-split-other-aria =
    .aria-label = [TH] Other payment method name [/TH]
payment-split-amount-aria =
    .aria-label = [TH] Split amount [/TH]
payment-split-amount-placeholder =
    .placeholder = [TH] 0.00 [/TH]
payment-split-remove-aria = [TH] Remove split [/TH]
    .aria-label = [TH] Remove split [/TH]
payment-split-remaining = [TH] Remaining [/TH]
payment-split-toggle = [TH] Split payment across methods [/TH]
payment-cancel = [TH] Cancel [/TH]
payment-open-bill = [TH] Open Bill [/TH]
payment-credit-sale = [TH] Credit Sale [/TH]
payment-customer-name = [TH] Customer Name [/TH]
payment-customer-name-label = [TH] Customer Name [/TH]
payment-customer-change = [TH] Change [/TH]
payment-customer-select = [TH] Select Customer [/TH]
payment-loyalty-use-points = [TH] Use Points [/TH]
payment-loyalty-points-label = [TH] Points [/TH]
payment-customer-search-heading = [TH] Select Customer [/TH]
payment-customer-search-loading = [TH] Loading… [/TH]
payment-customer-search-empty = [TH] No customers found [/TH]
payment-complete = [TH] Complete [/TH]
payment-retry-aria =
    .aria-label = [TH] Retry payment [/TH]
payment-retry = [TH] Retry [/TH]
payment-toast-currency-failed = [TH] Failed to load currency data [/TH]
payment-customer-placeholder =
    .placeholder = [TH] e.g. John Doe [/TH]
payment-loyalty-points-aria =
    .aria-label = [TH] Points [/TH]
payment-search-customers-aria = [TH] Search customers [/TH]
payment-search-customers-placeholder = [TH] Search by name, phone, or email... [/TH]

# ── Stock Shortfall Dialog ──
shortfall-dialog-aria =
    .aria-label = [TH] Insufficient stock resolution [/TH]
shortfall-title = [TH] Insufficient Stock [/TH]
shortfall-description = [TH] Some items don&apos;t have enough stock at the primary location. Choose alternative sources below. [/TH]
shortfall-wanted = [TH] Wanted [/TH]
shortfall-available = [TH] Available [/TH]
shortfall-alternatives-label = [TH] Alternative locations: [/TH]
shortfall-alt-available = [TH] available [/TH]
shortfall-split-qty-aria =
    .aria-label = [TH] Quantity from this location [/TH]
shortfall-simple-mode = [TH] Use single location [/TH]
shortfall-split-mode = [TH] Split across locations [/TH]
shortfall-no-alternatives = [TH] No alternative locations with stock available. [/TH]
shortfall-negative-override = [TH] Allow negative stock (Manager PIN override) [/TH]
shortfall-warehouse-warning = [TH] ⚠ Warehouse fulfillment may incur delivery charges. [/TH]
shortfall-cancel-btn = [TH] Cancel Sale [/TH]
shortfall-confirm-btn = [TH] Confirm &amp; Continue [/TH]

# Sales History
sales-history-title = [TH] Sales History [/TH]
sales-history-loading = [TH] Loading sales… [/TH]
sales-history-empty = [TH] No sales recorded yet [/TH]
sales-history-empty-filtered = [TH] No sales match your filters [/TH]
sales-history-count = [TH] { $count } sale{ $count -> [/TH]
  [one] 
  *[other] s
}
sales-history-page-info = [TH] Page { $current } of { $total } [/TH]
sales-history-col-id = [TH] Sale ID [/TH]
sales-history-col-date = [TH] Date [/TH]
sales-history-col-total = [TH] Total [/TH]
sales-history-col-items = [TH] Items [/TH]
sales-history-col-status = [TH] Status [/TH]
sales-history-col-payment = [TH] Payment [/TH]
sales-history-col-cashier = [TH] Cashier [/TH]
sales-history-view-aria = [TH] View { $id } [/TH]
sales-history-void-aria = [TH] Void order { $id } [/TH]
sales-history-search-placeholder =
    .placeholder = [TH] Search sale ID, payment, cashier… [/TH]
sales-history-search-aria =
    .aria-label = [TH] Search sales [/TH]
sales-history-filter-aria =
    .aria-label = [TH] Filter sales [/TH]
sales-history-status-filter-aria =
    .aria-label = [TH] Filter by status [/TH]
sales-history-date-from-aria =
    .aria-label = [TH] From date [/TH]
sales-history-date-to-aria =
    .aria-label = [TH] To date [/TH]
sales-history-cashier-aria =
    .aria-label = [TH] Filter by cashier [/TH]
sales-history-table-aria =
    .aria-label = [TH] Sales history [/TH]
sales-history-prev-aria =
    .aria-label = [TH] Previous page [/TH]
sales-history-next-aria =
    .aria-label = [TH] Next page [/TH]
sales-history-per-page-aria =
    .aria-label = [TH] Results per page [/TH]
sales-history-void-overlay-aria =
    .aria-label = [TH] Void order [/TH]
sales-history-void-reason-aria =
    .aria-label = [TH] Void reason [/TH]
sales-history-detail-overlay-aria =
    .aria-label = [TH] Sale detail [/TH]
sales-history-detail-close-aria =
    .aria-label = [TH] Close [/TH]
sales-history-lines-aria =
    .aria-label = [TH] Sale line items [/TH]
sales-history-actions-aria =
    .aria-label = [TH] Actions [/TH]
sales-history-pagination-aria =
    .aria-label = [TH] Pagination [/TH]
sales-history-void-close-aria =
    .aria-label = [TH] Close void dialog [/TH]
sales-history-refund-lines-aria =
    .aria-label = [TH] Refund line items [/TH]
sales-history-detail-title = [TH] Sale Detail [/TH]
sales-history-detail-close = [TH] Close [/TH]
sales-history-detail-print = [TH] Reprint Receipt [/TH]
sales-history-detail-id = [TH] ID [/TH]
sales-history-detail-date = [TH] Date [/TH]
sales-history-detail-status = [TH] Status [/TH]
sales-history-detail-payment = [TH] Payment [/TH]
sales-history-detail-cashier = [TH] Cashier [/TH]
sales-history-detail-subtotal = [TH] Subtotal [/TH]
sales-history-detail-tax = [TH] Tax [/TH]
sales-history-detail-total = [TH] Total [/TH]
sales-history-lines-title = [TH] Line Items [/TH]
sales-history-line-sku = [TH] SKU [/TH]
sales-history-line-name = [TH] Name [/TH]
sales-history-line-qty = [TH] Qty [/TH]
sales-history-line-unit-price = [TH] Unit Price [/TH]
sales-history-line-total = [TH] Total [/TH]
sales-history-line-tax = [TH] Tax [/TH]
sales-history-status-all = [TH] All [/TH]
sales-history-status-completed = [TH] Completed [/TH]
sales-history-status-pending = [TH] Pending [/TH]
sales-history-status-cancelled = [TH] Cancelled [/TH]
sales-history-status-voided = [TH] Voided [/TH]
sales-history-status-refunded = [TH] Refunded [/TH]
sales-history-export-csv = [TH] Export CSV [/TH]
sales-history-search-label = [TH] Search [/TH]
sales-history-status-label = [TH] Status [/TH]
sales-history-from-label = [TH] From [/TH]
sales-history-to-label = [TH] To [/TH]
sales-history-cashier-label = [TH] Cashier [/TH]
sales-history-cashier-all = [TH] All Cashiers [/TH]
sales-history-clear-filters = [TH] Clear filters [/TH]
sales-history-prev-page = [TH] ← Prev [/TH]
sales-history-next-page = [TH] Next → [/TH]
sales-history-per-page-label = [TH] Per page [/TH]
sales-history-void-title = [TH] Void Order [/TH]
sales-history-void-desc = [TH] This will cancel order { $id } for { $amount } and restore inventory. This action cannot be undone. [/TH]
sales-history-void-reason-label = [TH] Reason for void [/TH]
sales-history-void-cancel = [TH] Cancel [/TH]
sales-history-void-confirm = [TH] Confirm Void [/TH]
sales-history-void-progress = [TH] Voiding… [/TH]
sales-history-detail-loading = [TH] Loading… [/TH]
sales-history-action-view = [TH] View [/TH]
sales-history-action-void = [TH] Void [/TH]
sales-history-void-reason-placeholder =
    .placeholder = [TH] e.g. Customer cancellation [/TH]
sales-history-void-default-reason = [TH] Voided from sales history [/TH]
sales-history-void-error = [TH] Failed to void order [/TH]

# Sales History export
sales-history-export-id = [TH] Sale ID [/TH]
sales-history-export-date = [TH] Date [/TH]
sales-history-export-total = [TH] Total [/TH]
sales-history-export-items = [TH] Items [/TH]
sales-history-export-status = [TH] Status [/TH]
sales-history-export-payment = [TH] Payment [/TH]
sales-history-export-cashier = [TH] Cashier [/TH]
sales-history-pull-to-refresh = [TH] Pull down to refresh [/TH]
sales-history-release-to-refresh = [TH] Release to refresh [/TH]

# Sales Dashboard
sales-dashboard-title = [TH] Sales Dashboard [/TH]
sales-dashboard-daily-total = [TH] Daily Total [/TH]
sales-dashboard-total-sales = [TH] Total Sales [/TH]
sales-dashboard-total-items = [TH] Total Items [/TH]
sales-dashboard-hourly-title = [TH] Sales by Hour [/TH]
sales-dashboard-hourly-header-hour = [TH] Hour [/TH]
sales-dashboard-hourly-header-sales = [TH] Sales [/TH]
sales-dashboard-hourly-header-total = [TH] Total [/TH]
sales-dashboard-loading = [TH] Loading… [/TH]
sales-dashboard-no-data = [TH] No data for today [/TH]
sales-dashboard-revenue-title = [TH] Revenue (14d) [/TH]
sales-dashboard-category-title = [TH] By Category [/TH]
sales-dashboard-heatmap-title = [TH] Busiest Hours [/TH]

# Void Orders
void-orders-title = [TH] Orders [/TH]
void-orders-search-placeholder =
    .placeholder = [TH] Search by order ID or payment method… [/TH]
void-orders-search-aria =
    .aria-label = [TH] Search orders [/TH]
void-orders-filter-status-aria =
    .aria-label = [TH] Filter by status [/TH]
void-orders-status-all = [TH] All [/TH]
void-orders-status-active = [TH] Active [/TH]
void-orders-status-completed = [TH] Completed [/TH]
void-orders-status-voided = [TH] Voided [/TH]
void-orders-status-pending = [TH] Pending [/TH]
void-orders-loading = [TH] Loading orders… [/TH]
void-orders-retry = [TH] Retry [/TH]
void-orders-empty-filtered = [TH] No orders match the current filters. [/TH]
void-orders-empty-none = [TH] No orders recorded yet. [/TH]
void-orders-table-aria =
    .aria-label = [TH] Orders [/TH]
void-orders-col-order-id = [TH] Order ID [/TH]
void-orders-col-date = [TH] Date [/TH]
void-orders-col-status = [TH] Status [/TH]
void-orders-col-total = [TH] Total [/TH]
void-orders-col-items = [TH] Items [/TH]
void-orders-col-payment = [TH] Payment [/TH]
void-orders-col-actions = [TH] Actions [/TH]
void-orders-col-actions-aria =
    .aria-label = [TH] Actions [/TH]
void-orders-view-aria =
    .aria-label = [TH] View order { $id } [/TH]
void-orders-view = [TH] View [/TH]
void-orders-void-aria =
    .aria-label = [TH] Void order { $id } [/TH]
void-orders-void = [TH] Void [/TH]
void-orders-back-aria =
    .aria-label = [TH] Back to orders list [/TH]
void-orders-back = [TH] Back to Orders [/TH]
void-orders-loading-detail = [TH] Loading order details… [/TH]
void-orders-not-found = [TH] Order not found. [/TH]
void-orders-go-back = [TH] Go back [/TH]
void-orders-detail-heading = [TH] Order { $id } [/TH]
void-orders-meta-date = [TH] Date [/TH]
void-orders-meta-payment = [TH] Payment [/TH]
void-orders-meta-total = [TH] Total [/TH]
void-orders-meta-items = [TH] Items [/TH]
void-orders-line-items-title = [TH] Line Items [/TH]
void-orders-line-items-aria =
    .aria-label = [TH] Order line items [/TH]
void-orders-line-sku = [TH] SKU [/TH]
void-orders-line-name = [TH] Name [/TH]
void-orders-line-qty = [TH] Qty [/TH]
void-orders-line-unit-price = [TH] Unit Price [/TH]
void-orders-line-total = [TH] Total [/TH]
void-orders-void-section-title = [TH] Void Order [/TH]
void-orders-void-description = [TH] This will cancel the order, refund the payment, and restore stock to inventory. [/TH]
void-orders-reason-label = [TH] Reason for void [/TH]
void-orders-reason-select = [TH] Select a reason… [/TH]
void-orders-reason-placeholder =
    .placeholder = [TH] Enter the reason for voiding this order… [/TH]
void-orders-reason-aria =
    .aria-label = [TH] Custom void reason [/TH]
void-orders-cancel = [TH] Cancel [/TH]
void-orders-confirm-voiding = [TH] Voiding… [/TH]
void-orders-confirm = [TH] Confirm Void [/TH]
void-orders-voided-notice = [TH] This order has been voided. [/TH]
void-orders-error-load = [TH] Failed to load orders [/TH]
void-orders-error-reason = [TH] Please select or enter a void reason [/TH]
void-orders-error-void = [TH] Failed to void order [/TH]
void-orders-success-voided = [TH] Order voided successfully. Stock has been restored. [/TH]
void-orders-reason-cancelled = [TH] Cancelled by customer [/TH]
void-orders-reason-wrong-items = [TH] Wrong items scanned [/TH]
void-orders-reason-duplicate = [TH] Duplicate order [/TH]
void-orders-reason-price-dispute = [TH] Price dispute [/TH]
void-orders-reason-payment-issue = [TH] Payment issue [/TH]
void-orders-reason-changed-mind = [TH] Customer changed mind [/TH]
void-orders-reason-manager-override = [TH] Manager override [/TH]
void-orders-reason-other = [TH] Other reason… [/TH]

# Refund
refund-title = [TH] Process Refund [/TH]
refund-done-title = [TH] Refund Processed [/TH]
refund-done-amount = [TH] Refunded: { $amount } [/TH]
refund-done = [TH] Done [/TH]
refund-dialog-aria =
    .aria-label = [TH] Process refund [/TH]
refund-close-aria =
    .aria-label = [TH] Cancel refund [/TH]
refund-sale-id = [TH] Sale: { $id } [/TH]
refund-sale-total = [TH] Total: { $amount } [/TH]
refund-sale-date = [TH] Date: { $date } [/TH]
refund-items-title = [TH] Select Items to Refund [/TH]
refund-item-aria =
    .aria-label = [TH] Refund { $sku } [/TH]
refund-qty-decrease-aria =
    .aria-label = [TH] Decrease refund quantity [/TH]
refund-qty-increase-aria =
    .aria-label = [TH] Increase refund quantity [/TH]
refund-reason-label = [TH] Reason * [/TH]
refund-reason-placeholder =
    .placeholder = [TH] e.g. Customer changed mind [/TH]
refund-reason-aria =
    .aria-label = [TH] Refund reason [/TH]
refund-note-label = [TH] Note (internal) [/TH]
refund-note-placeholder =
    .placeholder = [TH] Optional internal note [/TH]
refund-note-aria =
    .aria-label = [TH] Refund note [/TH]
refund-total-label = [TH] Refund Total [/TH]
refund-cancel = [TH] Cancel [/TH]
refund-submit = [TH] Process Refund [/TH]
refund-error = [TH] Refund failed [/TH]

# Sales History Refund Line Items
refund-previous-refunds = [TH] Previous Refunds [/TH]
refund-line-sku = [TH] SKU [/TH]
refund-line-qty = [TH] Qty [/TH]
refund-line-total = [TH] Total [/TH]
refund-action-refund = [TH] Refund [/TH]

# Item Modifier Modal
modifier-no-options = [TH] No options available [/TH]
modifier-free = [TH] Free [/TH]
modifier-base-price = [TH] Base price [/TH]
modifier-addons = [TH] Add-ons [/TH]
modifier-total = [TH] Total [/TH]
modifier-add-to-cart = [TH] Add to Cart [/TH]

# EOD Report
eod-title = [TH] End-of-Day Report [/TH]
eod-cashier-shifts = [TH] Cashier Shifts [/TH]
eod-shift-active = [TH] Shift in progress [/TH]
eod-shift-active-since = [TH] Active shift since [/TH]
eod-opening-balance = [TH] Opening balance [/TH]
eod-sales-this-shift = [TH] Sales this shift [/TH]
eod-closed-shifts = [TH] Closed Shifts Today [/TH]
eod-col-opened = [TH] Opened [/TH]
eod-col-closed = [TH] Closed [/TH]
eod-col-opening = [TH] Opening [/TH]
eod-col-counted = [TH] Counted [/TH]
eod-col-expected = [TH] Expected [/TH]
eod-col-diff = [TH] Diff [/TH]
eod-total = [TH] Total [/TH]
eod-tag-over = [TH] Over [/TH]
eod-tag-short = [TH] Short [/TH]
eod-cash-reconciliation = [TH] Cash Reconciliation [/TH]
eod-cash-total-opening = [TH] Total opening [/TH]
eod-cash-total-counted = [TH] Total counted [/TH]
eod-cash-total-expected = [TH] Total expected [/TH]
eod-cash-net-diff = [TH] Net difference [/TH]
eod-refresh = [TH] Refresh [/TH]
eod-refresh-aria = [TH] Refresh report [/TH]
eod-printing = [TH] Printing… [/TH]
eod-print = [TH] Print [/TH]
eod-print-aria = [TH] Print EOD report [/TH]
eod-loading = [TH] Loading report… [/TH]
eod-error = [TH] { $error } [/TH]
eod-error-fallback = [TH] Failed to load report [/TH]
eod-retry = [TH] Retry [/TH]
eod-empty-title = [TH] No sales data available for today. [/TH]
eod-empty-sub = [TH] Sales will appear here once transactions are completed. [/TH]
eod-kpi-revenue = [TH] Total Revenue [/TH]
eod-kpi-revenue-sub = [TH] { $count } completed { $count -> [/TH]
    [one] sale
    *[other] sales
}
eod-kpi-average = [TH] Average Sale [/TH]
eod-kpi-average-sub = [TH] per transaction [/TH]
eod-kpi-voids = [TH] Voids [/TH]
eod-kpi-voids-sub = [TH] { $amount } voided [/TH]
eod-kpi-discounts = [TH] Discounts Applied [/TH]
eod-kpi-discounts-sub = [TH] { $count } { $count -> [/TH]
    [one] sale with discount
    *[other] sales with discount
}
eod-kpi-discounts-none = [TH] No discounts applied [/TH]
eod-payment-breakdown = [TH] Payment Breakdown [/TH]
eod-payment-empty = [TH] No payment data [/TH]
eod-payment-count = [TH] { $count } { $count -> [/TH]
    [one] transaction
    *[other] transactions
}
eod-payment-bar-aria = [TH] { $method }: { $pct }% of revenue [/TH]
eod-hourly-title = [TH] Sales by Hour [/TH]
eod-hourly-empty = [TH] No hourly data [/TH]
eod-hourly-chart-aria = [TH] Hourly sales bar chart [/TH]
eod-hour-bar-aria-sales = [TH] { $hour }:00 — { $count } { $count -> [/TH]
    [one] sale
    *[other] sales
}, { $amount }
eod-hour-bar-aria-none = [TH] { $hour }:00 — No sales [/TH]
eod-summary-title = [TH] Today's Summary [/TH]
eod-summary-completed = [TH] Completed Sales [/TH]
eod-summary-revenue = [TH] Total Revenue [/TH]
eod-summary-voided-sales = [TH] Voided Sales [/TH]
eod-summary-voided-value = [TH] Voided Value [/TH]
eod-summary-discounts = [TH] Sales with Discounts [/TH]
eod-summary-payment-methods = [TH] Payment Methods Used [/TH]

pos-cart-add-discount = [TH] + Add Discount [/TH]
pos-cart-apply = [TH] Apply [/TH]
pos-cart-cancel = [TH] Cancel [/TH]
pos-cart-clear = [TH] Clear [/TH]
pos-cart-discount-label = [TH] Discount ({ $label }) [/TH]
pos-cart-hold = [TH] Hold [/TH]
pos-cart-label-placeholder =
    .placeholder = [TH] Label (optional) [/TH]
pos-cart-lock = [TH] Lock [/TH]
pos-cart-lock-aria =
    .aria-label = [TH] Lock terminal and log out [/TH]
pos-cart-lock-title = [TH] Lock terminal [/TH]
pos-cart-pct-placeholder =
    .placeholder = [TH] % [/TH]
pos-cart-removed = [TH] Removed { $name } [/TH]
pos-cart-subtotal = [TH] Subtotal [/TH]
pos-cart-undo = [TH] Undo [/TH]
pos-close-shift-counted-label = [TH] Counted cash in drawer [/TH]
pos-close-shift-counted-placeholder =
    .placeholder = [TH] e.g. 150.00 [/TH]
pos-close-shift-notes-label = [TH] Notes (optional) [/TH]
pos-close-shift-notes-placeholder =
    .placeholder = [TH] Any notes about this shift… [/TH]
pos-close-shift-opened = [TH] Opened [/TH]
pos-close-shift-opening-balance = [TH] Opening balance [/TH]
pos-close-shift-title = [TH] Close Shift [/TH]
pos-held-empty = [TH] No held orders. [/TH]
pos-held-orders = [TH] Held Orders [/TH]
pos-held-resume = [TH] Resume [/TH]
pos-hold-cancel = [TH] Cancel [/TH]
pos-hold-desc = [TH] Enter a name for this held order so you can find it later. [/TH]
pos-hold-label-placeholder =
    .placeholder = [TH] e.g. Customer waiting for manager [/TH]
pos-hold-title = [TH] Hold Current Order [/TH]
pos-login-desc = [TH] Please log in to use the POS. [/TH]
pos-login-required = [TH] Login Required [/TH]
pos-open-shift-balance-label = [TH] Opening balance [/TH]
pos-open-shift-balance-placeholder =
    .placeholder = [TH] e.g. 100.00 [/TH]
pos-open-shift-title = [TH] Open Shift [/TH]
pos-shift-card-sales = [TH] Card Sales [/TH]
pos-shift-cash-sales = [TH] Cash Sales [/TH]
pos-shift-closed-title = [TH] Shift Closed [/TH]
pos-shift-counted = [TH] Counted [/TH]
pos-shift-difference = [TH] Difference [/TH]
pos-shift-expected-cash = [TH] Expected Cash [/TH]
pos-shift-header-close = [TH] Close Shift [/TH]
pos-shift-header-close-aria =
    .aria-label = [TH] Close current shift [/TH]
pos-shift-header-open = [TH] Open Shift [/TH]
pos-shift-header-open-aria =
    .aria-label = [TH] Open a new shift [/TH]
pos-shift-loading = [TH] Loading shift… [/TH]
pos-shift-no-active = [TH] No active shift [/TH]
pos-shift-notes = [TH] Notes [/TH]
pos-shift-open-since = [TH] Shift open since { $time } [/TH]
pos-shift-summary-done = [TH] Done [/TH]

# Cart Tip (items 6-10)
pos-cart-tip-label = [TH] Add Tip [/TH]
pos-cart-tip-none = [TH] None [/TH]
pos-cart-tip-aria = [TH] Tip selection [/TH]
pos-cart-tip-segment-aria = [TH] Set tip to { $percent } percent [/TH]
pos-cart-tip-segment-zero-aria = [TH] No tip [/TH]
pos-cart-tip-line = [TH] Tip ({ $percent }%) [/TH]

# Cart Service Charge
pos-cart-service-toggle-label = [TH] Add { $percent }% service charge [/TH]
pos-cart-service-toggle-aria = [TH] Toggle service charge [/TH]
pos-cart-service-line = [TH] Service ({ $percent }%) [/TH]

# Persistent undo
pos-cart-undo-dismiss = [TH] Dismiss [/TH]
pos-cart-undo-dismiss-aria = [TH] Dismiss undo notification [/TH]
pos-shift-total-sales = [TH] Total Sales [/TH]
pos-shift-over = [TH] Over [/TH]
pos-shift-short = [TH] Short [/TH]

# POS shift bar
pos-shift-close-btn = [TH] Close [/TH]
pos-shift-open-btn = [TH] Open [/TH]
pos-shift-close-aria =
    .aria-label = [TH] Close current shift [/TH]
pos-shift-open-aria =
    .aria-label = [TH] Open a new shift [/TH]
pos-dismiss-error-aria =
    .aria-label = [TH] Dismiss error [/TH]

# POS cart
pos-cart-undo-btn = [TH] Undo [/TH]
pos-cart-clear-aria =
    .aria-label = [TH] Clear all items from cart [/TH]
pos-cart-charge-aria =
    .aria-label = [TH] Charge the customer [/TH]
pos-cart-open-bill = [TH] Open Bill [/TH]
pos-cart-open-bill-aria =
    .aria-label = [TH] Save as open bill [/TH]
pos-cart-open-bills = [TH] Open Bills [/TH]
pos-cart-open-bills-aria =
    .aria-label = [TH] View open bills [/TH]
pos-cart-table-label = [TH] Table # [/TH]
pos-cart-table-placeholder =
    .placeholder = [TH] No. [/TH]
pos-cart-table-aria = [TH] Table number [/TH]
pos-cart-options-collapse-aria =
    .aria-label = [TH] Collapse options [/TH]
pos-cart-options-expand-aria =
    .aria-label = [TH] Expand options [/TH]
pos-cart-discount-pct-aria =
    .aria-label = [TH] Discount percentage [/TH]
pos-cart-discount-label-aria =
    .aria-label = [TH] Discount label [/TH]
pos-cart-discount-remove-aria =
    .aria-label = [TH] Remove discount [/TH]
pos-cart-discount-cancel-aria =
    .aria-label = [TH] Cancel discount [/TH]

# Cart line items (dynamic)
pos-cart-line-aria = [TH] { $sku }, { $qty } × { $amount } [/TH]
pos-cart-line-decrease-aria = [TH] Decrease quantity of { $sku } [/TH]
pos-cart-line-qty-aria = [TH] Quantity: { $qty } [/TH]
pos-cart-line-increase-aria = [TH] Increase quantity of { $sku } [/TH]
pos-cart-line-remove-aria = [TH] Remove { $sku } from cart [/TH]
pos-cart-line-swipe-remove-aria = [TH] Remove { $sku } [/TH]

# Cart panel
pos-cart-panel-aria = [TH] Cart [/TH]

# Shift modal overlay labels
pos-close-shift-overlay-aria = [TH] Close shift [/TH]
pos-close-shift-balance-aria = [TH] Closing balance [/TH]
pos-close-shift-notes-aria = [TH] Shift notes [/TH]
pos-close-shift-summary-aria = [TH] Shift closed summary [/TH]
pos-open-shift-overlay-aria = [TH] Open shift [/TH]
pos-open-shift-balance-aria = [TH] Opening balance [/TH]
pos-open-bill-overlay-aria = [TH] Open bill [/TH]
pos-open-bills-overlay-aria = [TH] Open bills list [/TH]

# POS open bill modal
pos-open-bill-title = [TH] Open Bill [/TH]
pos-open-bill-desc = [TH] Enter the customer name for this open bill. [/TH]
pos-open-bill-placeholder =
    .placeholder = [TH] e.g. John Doe [/TH]
pos-open-bill-name-aria =
    .aria-label = [TH] Customer name [/TH]
pos-open-bill-cancel = [TH] Cancel [/TH]
pos-open-bill-saving = [TH] Saving… [/TH]
pos-open-bill-save = [TH] Save Open Bill [/TH]
pos-open-bills-title = [TH] Open Bills [/TH]
pos-open-bills-close-aria =
    .aria-label = [TH] Close open bills list [/TH]
pos-open-bills-empty = [TH] No open bills. [/TH]
pos-open-bills-resume = [TH] Resume [/TH]

# ── Retail POS screen ──
retail-store-name-fallback = [TH] TOKO [/TH]
retail-shift-label = [TH] Shift [/TH]
retail-no-shift = [TH] No shift [/TH]
retail-search-placeholder = [TH] Cari produk… [/TH]
retail-search-clear-aria = [TH] Clear search [/TH]
retail-recent-label = [TH] Recent [/TH]
retail-no-products = [TH] No products [/TH]
retail-no-products-match = [TH] No products match your search [/TH]
retail-sku-label = [TH] SKU [/TH]
retail-sku-placeholder = [TH] Scan or type barcode / SKU [/TH]
retail-sku-go = [TH] GO [/TH]
retail-cart-items =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
retail-cart-header-col = [TH] # [/TH]
retail-cart-header-item = [TH] Item [/TH]
retail-cart-header-qty = [TH] Qty [/TH]
retail-cart-header-price = [TH] @Price [/TH]
retail-cart-header-subtotal = [TH] Subtotal [/TH]
retail-undo-items-removed =
    { $count ->
        [one] { $count } item removed
       *[other] { $count } items removed
    }
retail-total-discount = [TH] Discount { $percent }% [/TH]
retail-total-tax = [TH] PPN [/TH]
retail-pay-button = [TH] Pay [/TH]
retail-discount-button = [TH] Diskon [/TH]
retail-resume-button = [TH] Resume [/TH]
retail-credit-reminders = [TH] Credit Reminders ({ $count }) [/TH]
retail-fn-void = [TH] Void [/TH]
retail-fn-diskon = [TH] Diskon [/TH]
retail-fn-cari = [TH] Cari [/TH]
retail-fn-history = [TH] History [/TH]
retail-fn-pelanggan = [TH] Pelanggan [/TH]
retail-fn-stok = [TH] Stok [/TH]
retail-fn-shift = [TH] Shift [/TH]
retail-fn-options = [TH] Options [/TH]
retail-open-shift-opening-label = [TH] Opening balance (Rp) [/TH]
retail-open-shift-opening = [TH] Opening… [/TH]
retail-shift-closed-cash-sales = [TH] Cash Sales: [/TH]
retail-shift-closed-expected-label = [TH] Expected: [/TH]
retail-shift-closed-difference-label = [TH] Difference: [/TH]
retail-credit-reminders-title = [TH] Credit Reminders [/TH]
retail-credit-no-outstanding = [TH] No outstanding credits [/TH]
retail-credit-col-customer = [TH] Customer [/TH]
retail-credit-col-amount = [TH] Amount [/TH]
retail-credit-col-date = [TH] Date [/TH]
retail-credit-settle = [TH] Settle [/TH]
retail-clear-cart-title = [TH] Clear Cart [/TH]
retail-clear-cart-confirm =
    Remove all { $count ->
        [one] { $count } item from the cart?
       *[other] { $count } items from the cart?
    }
retail-clear-cart-clear = [TH] Clear [/TH]
retail-discount-title = [TH] Discount [/TH]
retail-discount-pct-tab = [TH] % [/TH]
retail-discount-rp-tab = [TH] Rp [/TH]
retail-discount-pct-label = [TH] Discount (%) [/TH]
retail-discount-rp-label = [TH] Discount (Rp) [/TH]
retail-customer-search-title = [TH] Select Customer [/TH]
retail-customer-search-placeholder =
    .placeholder = [TH] Search by name, phone, or email... [/TH]
retail-customer-search-loading = [TH] Loading... [/TH]
retail-customer-search-empty = [TH] No customers found [/TH]
retail-customer-clear = [TH] Clear [/TH]
retail-qty-total = [TH] Total: [/TH]
retail-qty-add = [TH] Add [/TH]
retail-shortcuts-title = [TH] Keyboard Shortcuts [/TH]
retail-shortcut-pay = [TH] Pay / Charge [/TH]
retail-shortcut-clear = [TH] Clear cart (Void) [/TH]
retail-shortcut-discount = [TH] Discount [/TH]
retail-shortcut-hold = [TH] Hold / Resume order [/TH]
retail-shortcut-sku = [TH] Focus SKU input [/TH]
retail-shortcut-shift = [TH] Open / Close shift [/TH]
retail-shortcut-options = [TH] Options [/TH]
retail-shortcut-list = [TH] This shortcut list [/TH]
retail-shortcut-close = [TH] Close modal / Options [/TH]
retail-shortcut-fullscreen = [TH] Toggle Fullscreen [/TH]
retail-toast-failed-products = [TH] Failed to load products [/TH]
retail-toast-failed-categories = [TH] Failed to load categories [/TH]
retail-toast-failed-settings = [TH] Failed to load store settings [/TH]
retail-toast-open-shift-first = [TH] Open a shift first [/TH]
retail-toast-order-held = [TH] Order held [/TH]
retail-toast-failed-hold = [TH] Failed to hold order [/TH]
retail-toast-failed-resume = [TH] Failed to resume order [/TH]
retail-toast-sale-complete = [TH] Sale complete [/TH]
retail-toast-credit-settled = [TH] Credit settled [/TH]
retail-toast-failed-settle = [TH] Failed to settle credit [/TH]
retail-toast-failed-open-shift = [TH] Failed to open shift [/TH]
retail-toast-sales-history-soon = [TH] Sales history coming soon [/TH]
retail-toast-stock-inquiry-soon = [TH] Stock inquiry coming soon [/TH]
retail-toast-failed-load-held = [TH] Failed to load held carts [/TH]
retail-toast-held-cart-deleted = [TH] Held cart deleted [/TH]
retail-toast-failed-delete-held = [TH] Failed to delete held cart [/TH]
retail-held-carts-title = [TH] Held Carts [/TH]
retail-held-carts-empty = [TH] No held carts [/TH]
retail-fn-bar-aria = [TH] Function bar [/TH]
retail-page-nav-aria = [TH] Product pages [/TH]
retail-page-prev-aria = [TH] Previous page [/TH]
retail-page-next-aria = [TH] Next page [/TH]
retail-cart-qty-decrease-aria = [TH] Decrease quantity [/TH]
retail-cart-qty-increase-aria = [TH] Increase quantity [/TH]
retail-cart-remove-aria = [TH] Remove from cart [/TH]
retail-toast-insufficient-stock = [TH] Insufficient stock [/TH]
retail-low-stock-banner =
    { $count ->
        [one] { $count } product low on stock
       *[other] { $count } products low on stock
    }
retail-held-cart-delete-aria = [TH] Delete held cart [/TH]



# ── Scale indicator widget ────────────────────────────────────────────────────
scale-indicator-aria = [TH] Scale weight indicator [/TH]
scale-idle = [TH] Scale [/TH]
scale-read-error = [TH] Scale error [/TH]

# ── Retail POS shortcut keys ───────────────────────────────────────────────
retail-fn-quick-return = [TH] Quick Return [/TH]
retail-header-workspaces-title = [TH] Back to workspaces [/TH]
retail-header-workspaces-aria = [TH] Back to workspaces [/TH]

# ── Gift Cards ─────────────────────────────────────────────────────
gift-cards-loading = [TH] Loading... [/TH]
gift-cards-status-all = [TH] All Statuses [/TH]
gift-cards-status-active = [TH] Active [/TH]
gift-cards-status-frozen = [TH] Frozen [/TH]
gift-cards-status-redeemed = [TH] Redeemed [/TH]
gift-cards-status-expired = [TH] Expired [/TH]
gift-cards-info-initial-balance = [TH] Initial Balance [/TH]
gift-cards-info-issued = [TH] Issued [/TH]
gift-cards-info-expires = [TH] Expires [/TH]
gift-cards-freeze = [TH] Freeze [/TH]
gift-cards-unfreeze = [TH] Unfreeze [/TH]
gift-cards-top-up = [TH] Top Up [/TH]
gift-cards-confirm-topup = [TH] Confirm Top-Up [/TH]
gift-cards-cancel-topup = [TH] Cancel [/TH]
gift-cards-recent-transactions = [TH] Recent Transactions [/TH]
gift-cards-txn-type = [TH] Type [/TH]
gift-cards-txn-amount = [TH] Amount [/TH]
gift-cards-txn-balance = [TH] Balance [/TH]
gift-cards-txn-notes = [TH] Notes [/TH]
gift-cards-txn-date = [TH] Date [/TH]

# Dashboard

