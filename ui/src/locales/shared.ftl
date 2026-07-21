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
locale-th = ไทย

# Accessibility
a11y-skip-to-content = Skip to main content

# Navigation section labels
nav-section-operations = Operations
nav-section-sales = Sales
nav-section-products = Products
nav-section-finance = Finance
nav-section-customers = Customers
nav-section-reports = Reports
nav-section-management = Management
nav-section-inventory = Inventory
nav-section-settings = Settings
nav-section-dev = Dev

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
nav-general = General
nav-dashboard-report = Dashboard
nav-sales-report = Sales Report
nav-inventory-report = Inventory Report
nav-design-system = Design System
nav-tooltip-preview = Tooltip Preview
nav-kiosk = Kiosk
nav-tables = Tables
nav-promotions = Promotions
nav-suppliers = Suppliers
nav-purchase-orders = Purchase Orders
nav-stock-transfers = Stock Transfers
nav-custom-report = Custom Report
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
workspace-home-fullscreen-aria = Toggle fullscreen
workspace-home-loading = Loading workspaces…
workspace-home-subtitle = Select a workspace to start
workspace-home-empty = No workspaces available
workspace-home-empty-desc = You don't have access to any workspaces yet. Contact an administrator.
workspace-card-open-aria = Open { $name }
workspace-card-no-access-aria = { $name } — not available for your role
workspace-card-no-access-title = Your role ({ $role }) cannot access this workspace
workspace-card-no-access-badge = Not available
workspace-home-logout = Logout
workspace-home-logout-confirm-title = Logout?
workspace-home-logout-confirm-desc = You will be returned to the login screen. Any unsaved work will be lost.
workspace-home-logout-confirm-cancel = Cancel
workspace-home-logout-confirm-confirm = Logout
workspace-home-shortcut-hint = Press { $key } to open
workspace-home-user-aria = Logged in as { $name }
workspace-home-error-title = Connection Error
workspace-home-error-desc = Could not load your workspaces. Check your connection and try again.
workspace-home-retry = Try Again
workspace-home-retry-btn = Retry
workspace-card-pin-aria = Pin { $name } to top
workspace-card-unpin-aria = Unpin { $name }
workspace-card-pinned-badge = Pinned

# Shell
shell-loading = Loading…

# Status Bar
status-bar-connected = Backend connected
status-bar-disconnected = Backend disconnected
status-bar-checking = Checking backend connection
status-bar-authenticating = Authenticating...
# P1-3: Tooltip for conflict count badge in StatusBar
statusbar-conflict-count = { $count } sync conflict(s) resolved

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

# Auth / License Activation
auth-activate-title = Activate License
auth-activate-subtitle = Enter your information below
auth-email-label = Email Address
auth-email-placeholder = store@example.com
auth-phone-label = Phone Number
auth-phone-placeholder = 08123456789
auth-license-label = License Key
auth-license-placeholder = OZ-PRO-XXXX-XXXX-XXXX
auth-activate-button = Activate License
auth-activating = Activating...
auth-activation-success = License activated successfully!
auth-activation-failed = Failed to activate license.
auth-activation-error = An error occurred during activation.
auth-validation-required = License key and Email are required.
auth-validation-invalid-email = Invalid email format.
auth-validation-phone-required = Phone number is required.
auth-validation-invalid-phone = Invalid phone number format. Enter at least 7 digits.
auth-paste = Paste
auth-version = Version { $version }
auth-ip-address = IP Address : { $ip }
auth-copyright = OZ-POS © { $year } All rights reserved.
auth-clipboard-error = Clipboard error: { $message }
auth-error-title = Error

## Create Owner PIN (first-run setup)
auth-create-pin-title = Create Owner PIN
auth-create-pin-desc = Set up the first owner account to manage your POS
auth-create-pin-display-name-label = Display Name
auth-create-pin-display-name-placeholder = Store Owner
auth-create-pin-username-label = Username
auth-create-pin-username-placeholder = owner
auth-create-pin-pin-label = PIN
auth-create-pin-pin-placeholder = At least 4 digits
auth-create-pin-confirm-label = Confirm PIN
auth-create-pin-confirm-placeholder = Re-enter PIN
auth-create-pin-creating = Creating...
auth-create-pin-create = Create Owner Account
auth-create-pin-success = Owner account created successfully!
auth-create-pin-error-fields = All fields are required.
auth-create-pin-error-pin-length = PIN must be at least 4 characters.
auth-create-pin-error-pin-mismatch = PINs do not match.
auth-create-pin-error-generic = An error occurred while creating the owner account.
