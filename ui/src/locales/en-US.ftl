# ui/src/locales/en-US.ftl — English strings for the OZ-POS front-end.
#
# IDs are `feature-element[-qualifier]`. Adding a new locale?
# Copy this file, translate, and register the bundle in src/main.tsx.

cart-title = Cart
cart-empty = Cart is empty
cart-line-remove = Remove
cart-total-label = Total

sale-pay-button = Pay
sale-pay-button-aria = Charge the customer for the current cart

cart-line-add-sample = Add sample line
cart-line-add-sample-aria = Add a sample product to the cart for testing

# Design system showcase
ds-title = Design System
theme-toggle-label = Toggle theme

# Product Lookup
product-lookup-title = Products
product-lookup-search-placeholder = Search products…
product-lookup-barcode-placeholder = Scan barcode…
product-lookup-barcode-scan = Scan
product-lookup-no-results = No products found
product-lookup-loading = Loading products…
product-lookup-add = Add to cart
product-lookup-in-stock = In stock
product-lookup-out-of-stock = Out of stock
product-lookup-all-categories = All Categories

# Setup Wizard
setup-title = OZ-POS
setup-tagline = Point of Sale — Simplified
setup-step-store-type = Store Type
setup-step-payments = Payments
setup-step-products = Products
setup-step-staff = Staff
setup-step-hardware = Hardware
setup-step-business-rules = Business Rules
setup-step-data-cloud = Data & Cloud
setup-step-review = Review

setup-preset-title = What kind of store are you running?
setup-preset-desc = Choose a preset that matches your business. You can customise every feature in the next steps.

setup-preset-simple-retail = Simple Retail
setup-preset-simple-retail-desc = Barcode scan, cash, receipt, inventory, tax — all essentials
setup-preset-restaurant = Restaurant
setup-preset-restaurant-desc = Tables, KDS, discounts, staff login — built for dining
setup-preset-full-store = Full Store
setup-preset-full-store-desc = Everything except cloud — payments, staff, loyalty, reports
setup-preset-custom = Custom
setup-preset-custom-desc = Start from scratch — enable exactly what you need

setup-nav-back = Back
setup-nav-next = Next
setup-nav-complete = Complete Setup
setup-nav-skip = Skip setup
setup-nav-skip-aria = Skip the setup wizard and use default settings

setup-features-desc = Toggle the features you need. You can change these later in Settings.

setup-review-title = Review Your Setup
setup-review-desc = Here&rsquo;s a summary of your choices. You can go back to change anything, or complete the setup.
setup-review-preset = Preset
setup-review-enabled = Enabled Features
setup-review-disabled = Disabled Features
setup-review-none = None
setup-review-everything-on = Everything on!
setup-review-more = +{ $count } more

setup-complete-title = All Set!
setup-complete-desc = Your { $preset } POS is configured with { $count } features enabled. You can change any setting later in Preferences.
setup-complete-launch = Launch OZ-POS

# POS Screen
pos-title = POS Terminal
pos-cart-panel-title = Current Sale
pos-cart-empty = Cart is empty
pos-cart-total = Total
pos-cart-qty-label = Qty
pos-cart-remove = Remove
pos-cart-pay = Charge { $amount }

# Badge
badge-default = Badge
badge-success = Success
badge-warning = Warning
badge-danger = Danger
badge-info = Info

# Spinner
spinner-label = Loading…

# Toast
toast-success = Operation completed successfully
toast-error = Something went wrong
toast-warning = Please check your input
toast-info = This is an informational message

# Empty state
empty-state-title = Nothing here yet
empty-state-desc = Get started by adding your first item
empty-state-cta = Add Product

# Error state
error-state-title = Something went wrong
error-state-desc = An unexpected error occurred. Please try again.
error-state-retry = Retry

# Audit Log
audit-log-load-more = Load More
audit-log-loading = Loading…

nav-inventory = Inventory

# Product Management
product-mgmt-title = Products
product-mgmt-add = Add Product
product-mgmt-loading = Loading products…
product-mgmt-empty = No products yet.
product-mgmt-empty-cta = Add your first product
product-mgmt-col-sku = SKU
product-mgmt-col-name = Name
product-mgmt-col-category = Category
product-mgmt-col-price = Price
product-mgmt-col-barcode = Barcode
product-mgmt-col-stock = Stock
product-mgmt-stock-in = In stock
product-mgmt-stock-out = Out of stock
product-mgmt-edit = Edit
product-mgmt-edit-aria = Edit { $name }
product-mgmt-delete = Delete
product-mgmt-delete-aria = Delete { $name }
product-mgmt-deleting =
    { $count ->
        [one] Deleting…
       *[other] …
    }
product-mgmt-modal-add-title = Add Product
product-mgmt-modal-edit-title = Edit Product
product-mgmt-modal-close = Close
product-mgmt-field-sku = SKU
product-mgmt-field-sku-required = SKU *
product-mgmt-field-name = Name
product-mgmt-field-name-required = Name *
product-mgmt-field-price = Price (minor units)
product-mgmt-field-currency = Currency
product-mgmt-field-category = Category
product-mgmt-field-barcode = Barcode
product-mgmt-field-tax-rates = Tax Rates
product-mgmt-field-stock = Initial stock
product-mgmt-btn-cancel = Cancel
product-mgmt-btn-create = Create
product-mgmt-btn-update = Update

# Sales History
sales-history-title = Sales History
sales-history-loading = Loading sales…
sales-history-empty = No sales recorded yet
sales-history-col-id = Sale ID
sales-history-col-date = Date
sales-history-col-total = Total
sales-history-col-items = Items
sales-history-col-status = Status
sales-history-col-payment = Payment
sales-history-view-aria = View { $id }
sales-history-detail-title = Sale Detail
sales-history-detail-close = Close
sales-history-detail-print = Reprint Receipt
sales-history-status-completed = Completed
sales-history-status-pending = Pending
sales-history-status-cancelled = Cancelled
sales-history-export-csv = Export CSV

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

# Tax Configuration
tax-config-title = Tax Configuration
tax-config-add = Add Tax Rate
tax-config-empty = No tax rates configured
tax-config-loading = Loading tax rates…
tax-config-col-name = Name
tax-config-col-rate = Rate (%)
tax-config-modal-title = { $editing ->
    [true] Edit Tax Rate
   *[other] Add Tax Rate
}
tax-config-field-name = Tax Name
tax-config-field-rate = Rate (%)
tax-config-btn-cancel = Cancel
tax-config-btn-save = Save
tax-config-btn-delete = Delete
tax-config-edit = Edit

# Exchange Rates
currency-title = Exchange Rates
currency-add = Add Exchange Rate
currency-loading = Loading exchange rates…
currency-empty = No exchange rates configured
currency-col-from = From
currency-col-to = To
currency-col-rate = Rate
currency-col-source = Source
currency-col-effective = Effective Date
currency-delete = Delete
currency-delete-confirm = Are you sure you want to delete this exchange rate?
currency-btn-cancel = Cancel
currency-btn-save = Save
currency-btn-add = Add
currency-modal-title = Add Exchange Rate
currency-field-from = From Currency
currency-field-to = To Currency
currency-field-rate = Rate
currency-field-source = Source (optional)
currency-field-date = Effective Date
currency-source-manual = manual

# Inventory Adjustment
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

# Staff Management
staff-title = Staff
staff-add-button = Add Staff
staff-loading = Loading staff…
staff-empty = No staff members yet.
staff-empty-cta = Add your first staff member
staff-col-name = Name
staff-col-username = Username
staff-col-role = Role
staff-col-status = Status
staff-col-actions =
    .aria-label = Actions
staff-status-active = Active
staff-status-inactive = Inactive
staff-edit = Edit
staff-edit-aria =
    .aria-label = Edit { $name }
staff-deactivate = Deactivate
staff-deactivate-aria =
    .aria-label = Deactivate { $name }
staff-restore = Restore
staff-restore-aria =
    .aria-label = Reactivate { $name }
staff-modal-add-aria =
    .aria-label = Add staff member
staff-modal-edit-aria =
    .aria-label = Edit staff member
staff-modal-add-title = Add Staff Member
staff-modal-edit-title = Edit Staff Member
staff-modal-close =
    .aria-label = Close
staff-field-username-label = Username *
staff-username-placeholder =
    .placeholder = e.g. jane
staff-field-name-label = Display Name *
staff-name-placeholder =
    .placeholder = e.g. Jane Smith
staff-field-pin-edit-label = New PIN (leave blank to keep current)
staff-field-pin-label = PIN * (4+ characters)
staff-pin-edit-placeholder =
    .placeholder = Leave blank to keep current
staff-pin-placeholder =
    .placeholder = Enter PIN
staff-field-role-label = Role *
staff-role-select-default = Select a role…
staff-btn-cancel = Cancel
staff-btn-update = Update
staff-btn-create = Create
staff-error-generic = { $message }
