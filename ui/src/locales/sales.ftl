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
pos-cart-empty-subtitle = Tap a menu item to start the order
pos-cart-total = Total
pos-cart-qty-label = Qty
pos-cart-remove = Remove
pos-cart-pay = Charge
pos-login-required-title = Login Required
pos-login-required-message = Please log in to use the POS.

# Bundle Expansion
pos-bundle-expanded =
    { $count ->
        [one] Bundle "{ $name }" added — 1 item to cart
       *[other] Bundle "{ $name }" added — { $count } items to cart
    }
pos-no-barcode-match = No product or bundle matches this barcode
pos-close-shift-cart-error = Complete or clear the current sale before closing the shift.
pos-close-shift-failed = Failed to close shift

# Scanner
pos-scanner-error = Scanner error: { $detail }

# Payment Modal
payment-dialog-aria =
    .aria-label = Payment
payment-title = Complete Order
payment-close-aria =
    .aria-label = Cancel payment
payment-done-title = Sale Complete
payment-change-label = Change due
payment-done-receipt = Receipt printed
payment-total-due = Total Due
payment-currency-aria =
    .aria-label = Charge currency
payment-currency-label = Charge Currency
payment-currency-select-aria =
    .aria-label = Select charge currency
payment-exchange-aria =
    .aria-label = Exchange rate information
payment-exchange-rate = Exchange rate
payment-rate-source = Rate source
payment-rate-timestamp = Rate timestamp
payment-rate-source-manual = manual
payment-receipt-currency-aria =
    .aria-label = Receipt currency information
payment-charged-in = Charged in
payment-default-currency = Default currency
payment-base-amount = Base amount
payment-charge-amount = Charge amount
payment-method-label = Payment Method
payment-method-cash = Cash
payment-method-card = Card
payment-method-qris = QRIS
payment-other-placeholder =
    .placeholder = Other...
payment-other-aria =
    .aria-label = Other payment method name
payment-amount-tendered = Amount Tendered
payment-tendered-input =
    .placeholder = 0.00
    .aria-label = Amount tendered
payment-quick-tender-aria =
    .aria-label = Tender { $amount }
payment-tender-exact-aria =
    .aria-label = Tend exact amount
payment-tender-exact = Exact
payment-customer-name-aria =
    .aria-label = Customer name for open bill
payment-change = Change
payment-insufficient = Insufficient amount
payment-qris-description = Generate a QRIS QR code for the customer to scan with their payment app.
payment-qris-btn-aria =
    .aria-label = Generate QRIS QR code
payment-qris-pay = Pay with QR
payment-split-title = Split Payments
payment-split-evenly-aria =
    .aria-label = Split evenly
payment-split-evenly = Split Evenly
payment-split-add-aria =
    .aria-label = Add split
payment-split-add = + Add Split
payment-split-method-cash = Cash
payment-split-method-card = Card
payment-split-other-placeholder =
    .placeholder = Other
payment-split-other-aria =
    .aria-label = Other payment method name
payment-split-amount-aria =
    .aria-label = Split amount
payment-split-amount-placeholder =
    .placeholder = 0.00
payment-split-remove-aria = Remove split
    .aria-label = Remove split
payment-split-remaining = Remaining
payment-split-toggle = Split payment across methods
payment-cancel = Cancel
payment-open-bill = Open Bill
payment-credit-sale = Credit Sale
payment-customer-name = Customer Name
payment-customer-name-label = Customer Name
payment-customer-change = Change
payment-customer-select = Select Customer
payment-loyalty-use-points = Use Points
payment-loyalty-points-label = Points
payment-customer-search-heading = Select Customer
payment-customer-search-loading = Loading…
payment-customer-search-empty = No customers found
payment-complete = Complete

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
sales-history-search-placeholder =
    .placeholder = Search sale ID, payment, cashier…
sales-history-search-aria =
    .aria-label = Search sales
sales-history-filter-aria =
    .aria-label = Filter sales
sales-history-status-filter-aria =
    .aria-label = Filter by status
sales-history-date-from-aria =
    .aria-label = From date
sales-history-date-to-aria =
    .aria-label = To date
sales-history-cashier-aria =
    .aria-label = Filter by cashier
sales-history-table-aria =
    .aria-label = Sales history
sales-history-prev-aria =
    .aria-label = Previous page
sales-history-next-aria =
    .aria-label = Next page
sales-history-per-page-aria =
    .aria-label = Results per page
sales-history-void-overlay-aria =
    .aria-label = Void order
sales-history-void-reason-aria =
    .aria-label = Void reason
sales-history-detail-overlay-aria =
    .aria-label = Sale detail
sales-history-detail-close-aria =
    .aria-label = Close
sales-history-lines-aria =
    .aria-label = Sale line items
sales-history-actions-aria =
    .aria-label = Actions
sales-history-pagination-aria =
    .aria-label = Pagination
sales-history-void-close-aria =
    .aria-label = Close void dialog
sales-history-refund-lines-aria =
    .aria-label = Refund line items
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
sales-history-void-reason-placeholder =
    .placeholder = e.g. Customer cancellation
sales-history-void-default-reason = Voided from sales history
sales-history-void-error = Failed to void order

# Sales History export
sales-history-export-id = Sale ID
sales-history-export-date = Date
sales-history-export-total = Total
sales-history-export-items = Items
sales-history-export-status = Status
sales-history-export-payment = Payment
sales-history-export-cashier = Cashier

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

# Sales History Refund Line Items
refund-previous-refunds = Previous Refunds
refund-line-sku = SKU
refund-line-qty = Qty
refund-line-total = Total
refund-action-refund = Refund

# Item Modifier Modal
modifier-no-options = No options available
modifier-free = Free
modifier-base-price = Base price
modifier-addons = Add-ons
modifier-total = Total
modifier-add-to-cart = Add to Cart

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

pos-cart-add-discount = + Add Discount
pos-cart-apply = Apply
pos-cart-cancel = Cancel
pos-cart-clear = Clear
pos-cart-discount-label = Discount ({ $label })
pos-cart-hold = Hold
pos-cart-label-placeholder =
    .placeholder = Label (optional)
pos-cart-lock = Lock
pos-cart-lock-aria =
    .aria-label = Lock terminal and log out
pos-cart-lock-title = Lock terminal
pos-cart-pct-placeholder =
    .placeholder = %
pos-cart-removed = Removed { $name }
pos-cart-subtotal = Subtotal
pos-cart-undo = Undo
pos-close-shift-counted-label = Counted cash in drawer
pos-close-shift-counted-placeholder =
    .placeholder = e.g. 150.00
pos-close-shift-notes-label = Notes (optional)
pos-close-shift-notes-placeholder =
    .placeholder = Any notes about this shift…
pos-close-shift-opened = Opened
pos-close-shift-opening-balance = Opening balance
pos-close-shift-title = Close Shift
pos-held-empty = No held orders.
pos-held-orders = Held Orders
pos-held-resume = Resume
pos-hold-cancel = Cancel
pos-hold-desc = Enter a name for this held order so you can find it later.
pos-hold-label-placeholder =
    .placeholder = e.g. Customer waiting for manager
pos-hold-title = Hold Current Order
pos-login-desc = Please log in to use the POS.
pos-login-required = Login Required
pos-open-shift-balance-label = Opening balance
pos-open-shift-balance-placeholder =
    .placeholder = e.g. 100.00
pos-open-shift-title = Open Shift
pos-shift-card-sales = Card Sales
pos-shift-cash-sales = Cash Sales
pos-shift-closed-title = Shift Closed
pos-shift-counted = Counted
pos-shift-difference = Difference
pos-shift-expected-cash = Expected Cash
pos-shift-header-close = Close Shift
pos-shift-header-close-aria =
    .aria-label = Close current shift
pos-shift-header-open = Open Shift
pos-shift-header-open-aria =
    .aria-label = Open a new shift
pos-shift-loading = Loading shift…
pos-shift-no-active = No active shift
pos-shift-notes = Notes
pos-shift-open-since = Shift open since { $time }
pos-shift-summary-done = Done

# Cart Tip (items 6-10)
pos-cart-tip-label = Add Tip
pos-cart-tip-none = None
pos-cart-tip-aria = Tip selection
pos-cart-tip-segment-aria = Set tip to { $percent } percent
pos-cart-tip-segment-zero-aria = No tip
pos-cart-tip-line = Tip ({ $percent }%)

# Cart Service Charge
pos-cart-service-toggle-label = Add { $percent }% service charge
pos-cart-service-toggle-aria = Toggle service charge
pos-cart-service-line = Service ({ $percent }%)

# Persistent undo
pos-cart-undo-dismiss = Dismiss
pos-cart-undo-dismiss-aria = Dismiss undo notification
pos-shift-total-sales = Total Sales
pos-shift-over = Over
pos-shift-short = Short

# POS shift bar
pos-shift-close-btn = Close
pos-shift-open-btn = Open
pos-shift-close-aria =
    .aria-label = Close current shift
pos-shift-open-aria =
    .aria-label = Open a new shift
pos-dismiss-error-aria =
    .aria-label = Dismiss error

# POS cart
pos-cart-undo-btn = Undo
pos-cart-clear-aria =
    .aria-label = Clear all items from cart
pos-cart-charge-aria =
    .aria-label = Charge the customer
pos-cart-open-bill = Open Bill
pos-cart-open-bill-aria =
    .aria-label = Save as open bill
pos-cart-open-bills = Open Bills
pos-cart-open-bills-aria =
    .aria-label = View open bills
pos-cart-table-label = Table #
pos-cart-table-placeholder =
    .placeholder = No.
pos-cart-table-aria = Table number
pos-cart-options-collapse-aria =
    .aria-label = Collapse options
pos-cart-options-expand-aria =
    .aria-label = Expand options
pos-cart-discount-pct-aria =
    .aria-label = Discount percentage
pos-cart-discount-label-aria =
    .aria-label = Discount label
pos-cart-discount-remove-aria =
    .aria-label = Remove discount
pos-cart-discount-cancel-aria =
    .aria-label = Cancel discount

# Cart line items (dynamic)
pos-cart-line-aria = { $sku }, { $qty } × { $amount }
pos-cart-line-decrease-aria = Decrease quantity of { $sku }
pos-cart-line-qty-aria = Quantity: { $qty }
pos-cart-line-increase-aria = Increase quantity of { $sku }
pos-cart-line-remove-aria = Remove { $sku } from cart
pos-cart-line-swipe-remove-aria = Remove { $sku }

# Cart panel
pos-cart-panel-aria = Cart

# Shift modal overlay labels
pos-close-shift-overlay-aria = Close shift
pos-close-shift-balance-aria = Closing balance
pos-close-shift-notes-aria = Shift notes
pos-close-shift-summary-aria = Shift closed summary
pos-open-shift-overlay-aria = Open shift
pos-open-shift-balance-aria = Opening balance
pos-open-bill-overlay-aria = Open bill
pos-open-bills-overlay-aria = Open bills list

# POS open bill modal
pos-open-bill-title = Open Bill
pos-open-bill-desc = Enter the customer name for this open bill.
pos-open-bill-placeholder =
    .placeholder = e.g. John Doe
pos-open-bill-name-aria =
    .aria-label = Customer name
pos-open-bill-cancel = Cancel
pos-open-bill-saving = Saving…
pos-open-bill-save = Save Open Bill
pos-open-bills-title = Open Bills
pos-open-bills-close-aria =
    .aria-label = Close open bills list
pos-open-bills-empty = No open bills.
pos-open-bills-resume = Resume

# ── Retail POS screen ──
retail-store-name-fallback = TOKO
retail-shift-label = Shift
retail-no-shift = No shift
retail-search-placeholder = Cari produk…
retail-search-clear-aria = Clear search
retail-recent-label = Recent
retail-no-products = No products
retail-no-products-match = No products match your search
retail-sku-label = SKU
retail-sku-placeholder = Scan or type barcode / SKU
retail-sku-go = GO
retail-cart-items =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
retail-cart-header-col = #
retail-cart-header-item = Item
retail-cart-header-qty = Qty
retail-cart-header-price = @Price
retail-cart-header-subtotal = Subtotal
retail-undo-items-removed =
    { $count ->
        [one] { $count } item removed
       *[other] { $count } items removed
    }
retail-total-discount = Discount { $percent }%
retail-total-tax = PPN
retail-pay-button = Pay
retail-discount-button = Diskon
retail-resume-button = Resume
retail-credit-reminders = Credit Reminders ({ $count })
retail-fn-void = Void
retail-fn-diskon = Diskon
retail-fn-cari = Cari
retail-fn-history = History
retail-fn-pelanggan = Pelanggan
retail-fn-stok = Stok
retail-fn-shift = Shift
retail-fn-options = Options
retail-open-shift-opening-label = Opening balance (Rp)
retail-open-shift-opening = Opening…
retail-shift-closed-cash-sales = Cash Sales:
retail-shift-closed-expected-label = Expected:
retail-shift-closed-difference-label = Difference:
retail-credit-reminders-title = Credit Reminders
retail-credit-no-outstanding = No outstanding credits
retail-credit-col-customer = Customer
retail-credit-col-amount = Amount
retail-credit-col-date = Date
retail-credit-settle = Settle
retail-clear-cart-title = Clear Cart
retail-clear-cart-confirm =
    Remove all { $count ->
        [one] { $count } item from the cart?
       *[other] { $count } items from the cart?
    }
retail-clear-cart-clear = Clear
retail-discount-title = Discount
retail-discount-pct-tab = %
retail-discount-rp-tab = Rp
retail-discount-pct-label = Discount (%)
retail-discount-rp-label = Discount (Rp)
retail-customer-search-title = Select Customer
retail-customer-search-placeholder =
    .placeholder = Search by name, phone, or email...
retail-customer-search-loading = Loading...
retail-customer-search-empty = No customers found
retail-customer-clear = Clear
retail-qty-total = Total:
retail-qty-add = Add
retail-shortcuts-title = Keyboard Shortcuts
retail-shortcut-pay = Pay / Charge
retail-shortcut-clear = Clear cart (Void)
retail-shortcut-discount = Discount
retail-shortcut-hold = Hold / Resume order
retail-shortcut-sku = Focus SKU input
retail-shortcut-shift = Open / Close shift
retail-shortcut-options = Options
retail-shortcut-list = This shortcut list
retail-shortcut-close = Close modal / Options
retail-shortcut-fullscreen = Toggle Fullscreen
retail-toast-failed-products = Failed to load products
retail-toast-failed-categories = Failed to load categories
retail-toast-failed-settings = Failed to load store settings
retail-toast-open-shift-first = Open a shift first
retail-toast-order-held = Order held
retail-toast-failed-hold = Failed to hold order
retail-toast-failed-resume = Failed to resume order
retail-toast-sale-complete = Sale complete
retail-toast-credit-settled = Credit settled
retail-toast-failed-settle = Failed to settle credit
retail-toast-failed-open-shift = Failed to open shift
retail-toast-sales-history-soon = Sales history coming soon
retail-toast-stock-inquiry-soon = Stock inquiry coming soon
retail-toast-failed-load-held = Failed to load held carts
retail-toast-held-cart-deleted = Held cart deleted
retail-toast-failed-delete-held = Failed to delete held cart
retail-held-carts-title = Held Carts
retail-held-carts-empty = No held carts
retail-fn-bar-aria = Function bar
retail-page-nav-aria = Product pages
retail-page-prev-aria = Previous page
retail-page-next-aria = Next page
retail-cart-qty-decrease-aria = Decrease quantity
retail-cart-qty-increase-aria = Increase quantity
retail-cart-remove-aria = Remove from cart
retail-toast-insufficient-stock = Insufficient stock
retail-low-stock-banner =
    { $count ->
        [one] { $count } product low on stock
       *[other] { $count } products low on stock
    }
retail-held-cart-delete-aria = Delete held cart



# ── Scale indicator widget ────────────────────────────────────────────────────
scale-indicator-aria = Scale weight indicator
scale-idle = Scale
scale-read-error = Scale error

# ── Retail POS shortcut keys ───────────────────────────────────────────────
retail-fn-quick-return = Quick Return
retail-header-workspaces-title = Back to workspaces
retail-header-workspaces-aria = Back to workspaces

# ── Gift Cards ─────────────────────────────────────────────────────
gift-cards-loading = Loading...
gift-cards-status-all = All Statuses
gift-cards-status-active = Active
gift-cards-status-frozen = Frozen
gift-cards-status-redeemed = Redeemed
gift-cards-status-expired = Expired
gift-cards-info-initial-balance = Initial Balance
gift-cards-info-issued = Issued
gift-cards-info-expires = Expires
gift-cards-freeze = Freeze
gift-cards-unfreeze = Unfreeze
gift-cards-top-up = Top Up
gift-cards-confirm-topup = Confirm Top-Up
gift-cards-cancel-topup = Cancel
gift-cards-recent-transactions = Recent Transactions
gift-cards-txn-type = Type
gift-cards-txn-amount = Amount
gift-cards-txn-balance = Balance
gift-cards-txn-notes = Notes
gift-cards-txn-date = Date

# Dashboard
