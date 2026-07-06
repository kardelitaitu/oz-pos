# ui/src/locales/shared.ftl — Shared UI strings used across features
#
# IDs are `feature-element[-qualifier]`.

# Design system showcase
ds-title = Design System
theme-toggle-label = Toggle theme
theme-toggle-aria =
    .aria-label = Switch to { $mode ->
        [dark] dark
       *[light] light
    } mode

# Badge
badge-default = Badge
badge-success = Success
badge-warning = Warning
badge-danger = Danger
badge-info = Info

# Loading / Spinner
shared-loading = Loading…
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

# Error boundary
error-boundary-title = Something went wrong

# Error state
error-state-title = Something went wrong
error-state-desc = An unexpected error occurred. Please try again.
error-state-retry = Retry

# Navigation
nav-inventory = Inventory

# Common / Global
cancel = Cancel
save = Save
delete = Delete
edit = Edit
close = Close
loading = Loading…
print = Print
back = Back
retry = Retry
search = Search
no-results = No results found
error-occurred = An error occurred

# Audit Log
audit-log-title = Audit Log
audit-log-load-more = Load More
audit-log-loading = Loading…
audit-log-refresh = Refresh
audit-log-retry = Retry
audit-log-filter-all = All
audit-log-filter-success = Success
audit-log-filter-failure = Failure
audit-log-loading-text = Loading audit log…
audit-log-empty-filtered = No audit entries match the current filters.
audit-log-empty-none = No audit entries recorded yet. Entries appear when sales are completed, voided, or staff actions occur.
audit-log-col-date = Date
audit-log-col-action = Action
audit-log-col-target = Target
audit-log-col-user = User ID
audit-log-col-outcome = Outcome
audit-log-col-details = Details
audit-log-count = { $count } entr{ $count ->
  [one] y
  *[other] ies
}

# Update Banner
update-banner-title = Update available
update-banner-new-version = New version
update-banner-install = Install
update-banner-installing = Installing…
update-banner-install-aria = Download and install update
update-banner-installing-aria = Installing update…
update-banner-dismiss-aria = Dismiss update notification

# Toast
toast-dismiss-aria = Dismiss notification
toast-notifications-aria = Notifications

# Modal
modal-close-aria = Close dialog

# Permission Denied
permission-denied-title = Access Denied
permission-denied-desc = { $action } requires a { $requiredRole } role.
permission-denied-current = You are logged in as { $displayName } ({ $roleName }).
permission-denied-go-back = Go back

# Store Switcher
store-switcher-select = Select Store
store-switcher-current-aria = Current store: { $name }. Click to switch.
store-switcher-list-aria = Stores
store-switcher-primary = · Primary

# Gateway Status
gateway-status-online-aria = { $name } online
gateway-status-offline-aria = { $name } offline

# Role Badge
role-badge-logged-in-aria = Logged in as { $displayName }, { $roleName }
role-badge-logout-aria = Log out { $displayName }
role-badge-logout-title = Log out

# Language Selector
language-selector-label = Language
language-selector-select-aria = Select language

# Locale labels
locale-en = English
locale-id = Bahasa Indonesia

# Navigation
nav-pos-terminal = POS Terminal
nav-kds = KDS
nav-products = Products
nav-stock-adjust = Stock Adjust
nav-sales-history = Sales History
nav-dashboard = Dashboard
nav-eod-report = EOD Report
nav-orders = Orders
nav-tax-rates = Tax Rates
nav-exchange-rates = Exchange Rates
nav-categories = Categories
nav-customers = Customers
nav-loyalty = Loyalty
nav-staff = Staff
nav-terminals = Terminals
nav-stores = Stores
nav-features = Features
nav-data = Data
nav-audit-log = Audit Log
nav-offline-queue = Offline Queue
nav-shifts = Shifts
nav-bundles = Bundles
nav-settings = Settings
nav-dashboard-report = Dashboard
nav-sales-report = Sales Report
nav-inventory-report = Inventory Report
nav-design-system = Design System
nav-kiosk = Kiosk
nav-tables = Tables
nav-promotions = Promotions
nav-suppliers = Suppliers
nav-purchase-orders = Purchase Orders
nav-stock-transfers = Stock Transfers
nav-pos = POS
nav-stock = Stock
nav-history = History
nav-reports = Reports
nav-section-app = App
nav-sidebar-collapse = Collapse sidebar
nav-sidebar-expand = Expand sidebar
nav-main-aria = Main navigation
nav-tablist-aria = Navigation tabs
nav-switch-workspace = Switch Workspace

# Workspace home
workspace-home-loading = Loading workspaces…
workspace-home-subtitle = Select a workspace to start
workspace-home-empty = No workspaces available
workspace-home-empty-desc = You don't have access to any workspaces yet. Contact an administrator.
workspace-card-open-aria = Open { $name }
workspace-card-no-access-aria = { $name } — not available for your role
workspace-card-no-access-title = Your role ({ $role }) cannot access this workspace
workspace-card-no-access-badge = Not available
workspace-home-logout = Logout

# Shell
shell-loading = Loading…

# Status Bar
status-bar-connected = Backend connected
status-bar-disconnected = Backend disconnected
status-bar-checking = Checking backend connection
status-bar-authenticating = Authenticating...

# Audit Action Labels
audit-action-sale-void = Void Sale
audit-action-sale-complete = Complete Sale
audit-action-sale-refund = Refund
audit-action-login = Staff Login
audit-action-login-failed = Login Failed
audit-action-user-create = Staff Created
audit-action-user-update = Staff Updated
audit-action-product-create = Product Created
audit-action-product-update = Product Updated
audit-action-product-delete = Product Deleted
audit-action-stock-adjust = Stock Adjusted
audit-action-setting-change = Setting Changed
audit-action-system-backup = Backup Created
audit-action-system-restore = Restore
audit-action-system-export = Data Export
audit-action-system-import = Data Import
audit-log-table-label = Audit log entries
audit-log-search-placeholder = Search actions, targets, or users…
audit-log-search-label = Search audit log
audit-log-filter-label = Filter by outcome
