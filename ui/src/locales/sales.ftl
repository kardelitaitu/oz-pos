# ui/src/locales/sales.ftl — POS, cart, sales history, dashboard, refunds

# Cart
cart-title = Cart
cart-empty = Cart is empty
cart-line-remove = Remove
cart-total-label = Total
cart-line-add-sample = Add sample line
cart-line-add-sample-aria = Add a sample product to the cart for testing

# POS
sale-pay-button = Pay
sale-pay-button-aria = Charge the customer for the current cart
pos-title = POS Terminal
pos-cart-panel-title = Current Sale
pos-cart-empty = Cart is empty
pos-cart-total = Total
pos-cart-qty-label = Qty
pos-cart-remove = Remove
pos-cart-pay = Charge { $amount }
pos-login-required-title = Login Required
pos-login-required-message = Please log in to use the POS.

# Bundle Expansion
pos-bundle-expanded = Bundle added: { $count } item{ $count ->
  [one] 
  *[other] s
} to cart

# Scanner
pos-scanner-error = Scanner error: { $detail }

# Sales History
sales-history-title = Sales History
sales-history-loading = Loading sales…
sales-history-empty = No sales recorded yet
sales-history-empty-filtered = No sales match your filters
sales-history-count = { $count } sale{ $count ->
  [one] 
  *[other] s
}
sales-history-page-info = Page { $current } of { $total }
sales-history-col-id = Sale ID
sales-history-col-date = Date
sales-history-col-total = Total
sales-history-col-items = Items
sales-history-col-status = Status
sales-history-col-payment = Payment
sales-history-col-cashier = Cashier
sales-history-view-aria = View { $id }
sales-history-void-aria = Void order { $id }
sales-history-detail-title = Sale Detail
sales-history-detail-close = Close
sales-history-detail-print = Reprint Receipt
sales-history-detail-id = ID
sales-history-detail-date = Date
sales-history-detail-status = Status
sales-history-detail-payment = Payment
sales-history-detail-cashier = Cashier
sales-history-detail-subtotal = Subtotal
sales-history-detail-tax = Tax
sales-history-detail-total = Total
sales-history-lines-title = Line Items
sales-history-line-sku = SKU
sales-history-line-name = Name
sales-history-line-qty = Qty
sales-history-line-unit-price = Unit Price
sales-history-line-total = Total
sales-history-line-tax = Tax
sales-history-status-all = All
sales-history-status-completed = Completed
sales-history-status-pending = Pending
sales-history-status-cancelled = Cancelled
sales-history-status-voided = Voided
sales-history-status-refunded = Refunded
sales-history-export-csv = Export CSV
sales-history-search-label = Search
sales-history-status-label = Status
sales-history-from-label = From
sales-history-to-label = To
sales-history-cashier-label = Cashier
sales-history-cashier-all = All Cashiers
sales-history-clear-filters = Clear filters
sales-history-prev-page = ← Prev
sales-history-next-page = Next →
sales-history-per-page-label = Per page
sales-history-void-title = Void Order
sales-history-void-desc = This will cancel order { $id } for { $amount } and restore inventory. This action cannot be undone.
sales-history-void-reason-label = Reason for void
sales-history-void-cancel = Cancel
sales-history-void-confirm = Confirm Void
sales-history-void-progress = Voiding…
sales-history-detail-loading = Loading…
sales-history-action-view = View
sales-history-action-void = Void

# Sales Dashboard
sales-dashboard-title = Sales Dashboard
sales-dashboard-daily-total = Daily Total
sales-dashboard-total-sales = Total Sales
sales-dashboard-total-items = Total Items
sales-dashboard-hourly-title = Sales by Hour
sales-dashboard-hourly-header-hour = Hour
sales-dashboard-hourly-header-sales = Sales
sales-dashboard-hourly-header-total = Total
sales-dashboard-loading = Loading…
sales-dashboard-no-data = No data for today

# Void Orders
void-orders-title = Orders
void-orders-search-placeholder =
    .placeholder = Search by order ID or payment method…
void-orders-search-aria =
    .aria-label = Search orders
void-orders-filter-status-aria =
    .aria-label = Filter by status
void-orders-status-all = All
void-orders-status-active = Active
void-orders-status-completed = Completed
void-orders-status-voided = Voided
void-orders-status-pending = Pending
void-orders-loading = Loading orders…
void-orders-retry = Retry
void-orders-empty-filtered = No orders match the current filters.
void-orders-empty-none = No orders recorded yet.
void-orders-table-aria =
    .aria-label = Orders
void-orders-col-order-id = Order ID
void-orders-col-date = Date
void-orders-col-status = Status
void-orders-col-total = Total
void-orders-col-items = Items
void-orders-col-payment = Payment
void-orders-col-actions = Actions
void-orders-col-actions-aria =
    .aria-label = Actions
void-orders-view-aria =
    .aria-label = View order { $id }
void-orders-view = View
void-orders-void-aria =
    .aria-label = Void order { $id }
void-orders-void = Void
void-orders-back-aria =
    .aria-label = Back to orders list
void-orders-back = Back to Orders
void-orders-loading-detail = Loading order details…
void-orders-not-found = Order not found.
void-orders-go-back = Go back
void-orders-detail-heading = Order { $id }
void-orders-meta-date = Date
void-orders-meta-payment = Payment
void-orders-meta-total = Total
void-orders-meta-items = Items
void-orders-line-items-title = Line Items
void-orders-line-items-aria =
    .aria-label = Order line items
void-orders-line-sku = SKU
void-orders-line-name = Name
void-orders-line-qty = Qty
void-orders-line-unit-price = Unit Price
void-orders-line-total = Total
void-orders-void-section-title = Void Order
void-orders-void-description = This will cancel the order, refund the payment, and restore stock to inventory.
void-orders-reason-label = Reason for void
void-orders-reason-select = Select a reason…
void-orders-reason-placeholder =
    .placeholder = Enter the reason for voiding this order…
void-orders-reason-aria =
    .aria-label = Custom void reason
void-orders-cancel = Cancel
void-orders-confirm-voiding = Voiding…
void-orders-confirm = Confirm Void
void-orders-voided-notice = This order has been voided.
void-orders-error-load = Failed to load orders
void-orders-error-reason = Please select or enter a void reason
void-orders-error-void = Failed to void order
void-orders-success-voided = Order voided successfully. Stock has been restored.
void-orders-reason-cancelled = Cancelled by customer
void-orders-reason-wrong-items = Wrong items scanned
void-orders-reason-duplicate = Duplicate order
void-orders-reason-price-dispute = Price dispute
void-orders-reason-payment-issue = Payment issue
void-orders-reason-changed-mind = Customer changed mind
void-orders-reason-manager-override = Manager override
void-orders-reason-other = Other reason…

# Refund
refund-title = Process Refund
refund-done-title = Refund Processed
refund-done-amount = Refunded: { $amount }
refund-done = Done
refund-dialog-aria =
    .aria-label = Process refund
refund-close-aria =
    .aria-label = Cancel refund
refund-sale-id = Sale: { $id }
refund-sale-total = Total: { $amount }
refund-sale-date = Date: { $date }
refund-items-title = Select Items to Refund
refund-item-aria =
    .aria-label = Refund { $sku }
refund-qty-decrease-aria =
    .aria-label = Decrease refund quantity
refund-qty-increase-aria =
    .aria-label = Increase refund quantity
refund-reason-label = Reason *
refund-reason-placeholder =
    .placeholder = e.g. Customer changed mind
refund-reason-aria =
    .aria-label = Refund reason
refund-note-label = Note (internal)
refund-note-placeholder =
    .placeholder = Optional internal note
refund-note-aria =
    .aria-label = Refund note
refund-total-label = Refund Total
refund-cancel = Cancel
refund-submit = Process Refund
refund-error = Refund failed

# EOD Report
eod-title = End-of-Day Report
eod-cashier-shifts = Cashier Shifts
eod-shift-active = Shift in progress
eod-shift-active-since = Active shift since
eod-opening-balance = Opening balance
eod-sales-this-shift = Sales this shift
eod-closed-shifts = Closed Shifts Today
eod-col-opened = Opened
eod-col-closed = Closed
eod-col-opening = Opening
eod-col-counted = Counted
eod-col-expected = Expected
eod-col-diff = Diff
eod-total = Total
eod-tag-over = Over
eod-tag-short = Short
eod-cash-reconciliation = Cash Reconciliation
eod-cash-total-opening = Total opening
eod-cash-total-counted = Total counted
eod-cash-total-expected = Total expected
eod-cash-net-diff = Net difference
eod-refresh = Refresh
eod-refresh-aria = Refresh report
eod-printing = Printing…
eod-print = Print
eod-print-aria = Print EOD report
eod-loading = Loading report…
eod-error = { $error }
eod-error-fallback = Failed to load report
eod-retry = Retry
eod-empty-title = No sales data available for today.
eod-empty-sub = Sales will appear here once transactions are completed.
eod-kpi-revenue = Total Revenue
eod-kpi-revenue-sub = { $count } completed { $count ->
    [one] sale
    *[other] sales
}
eod-kpi-average = Average Sale
eod-kpi-average-sub = per transaction
eod-kpi-voids = Voids
eod-kpi-voids-sub = { $amount } voided
eod-kpi-discounts = Discounts Applied
eod-kpi-discounts-sub = { $count } { $count ->
    [one] sale with discount
    *[other] sales with discount
}
eod-kpi-discounts-none = No discounts applied
eod-payment-breakdown = Payment Breakdown
eod-payment-empty = No payment data
eod-payment-count = { $count } { $count ->
    [one] transaction
    *[other] transactions
}
eod-payment-bar-aria = { $method }: { $pct }% of revenue
eod-hourly-title = Sales by Hour
eod-hourly-empty = No hourly data
eod-hourly-chart-aria = Hourly sales bar chart
eod-hour-bar-aria-sales = { $hour }:00 — { $count } { $count ->
    [one] sale
    *[other] sales
}, { $amount }
eod-hour-bar-aria-none = { $hour }:00 — No sales
eod-summary-title = Today's Summary
eod-summary-completed = Completed Sales
eod-summary-revenue = Total Revenue
eod-summary-voided-sales = Voided Sales
eod-summary-voided-value = Voided Value
eod-summary-discounts = Sales with Discounts
eod-summary-payment-methods = Payment Methods Used
