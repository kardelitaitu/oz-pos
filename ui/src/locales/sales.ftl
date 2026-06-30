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

# Refunds
refund-title = Process Refund
refund-sale-info = Sale { $id } — { $total } on { $date }
refund-select-items = Select Items to Refund
refund-reason = Reason
refund-reason-placeholder = e.g. Customer changed mind
refund-reason-required = Reason is required
refund-note = Note (internal)
refund-note-placeholder = Optional internal note
refund-total = Refund Total
refund-process = Process Refund
refund-cancel = Cancel
refund-done-title = Refund Processed
refund-done-amount = Refunded: { $amount }
refund-action-refund = Refund
refund-error-failed = Refund failed. Please try again.
refund-no-refunds = No refunds for this sale.
refund-previous-refunds = Previous Refunds
refund-line-aria = Select { $sku } for refund
refund-decrease-aria = Decrease refund quantity
refund-increase-aria = Increase refund quantity
refund-line-sku = SKU
refund-line-qty = Qty
refund-line-total = Total
