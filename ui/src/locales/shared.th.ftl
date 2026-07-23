# ui/src/locales/shared.ftl — Shared UI strings used across features
#
# IDs are `feature-element[-qualifier]`.

# Design system showcase
ds-title = [TH] Design System [/TH]
theme-toggle-label = [TH] Toggle theme [/TH]
theme-toggle-aria =
    .aria-label = [TH] Switch to { $mode -> [/TH]
        [dark] dark
       *[light] light
    } mode

# Badge
badge-default = [TH] Badge [/TH]
badge-success = [TH] Success [/TH]
badge-warning = [TH] Warning [/TH]
badge-danger = [TH] Danger [/TH]
badge-info = [TH] Info [/TH]

# Loading / Spinner
shared-loading = [TH] Loading… [/TH]
spinner-label = [TH] Loading… [/TH]

# Toast
toast-success = [TH] Operation completed successfully [/TH]
toast-error = [TH] Something went wrong [/TH]
toast-warning = [TH] Please check your input [/TH]
toast-info = [TH] This is an informational message [/TH]

# Empty state
empty-state-title = [TH] Nothing here yet [/TH]
empty-state-desc = [TH] Get started by adding your first item [/TH]
empty-state-cta = [TH] Add Product [/TH]

# Error boundary
error-boundary-title = [TH] Something went wrong [/TH]

# Error state
error-state-title = [TH] Something went wrong [/TH]
error-state-desc = [TH] An unexpected error occurred. Please try again. [/TH]
error-state-retry = [TH] Retry [/TH]

# Navigation
nav-inventory = [TH] Inventory [/TH]

# Common / Global
cancel = [TH] Cancel [/TH]
confirm = [TH] Confirm [/TH]
save = [TH] Save [/TH]
delete = [TH] Delete [/TH]
edit = [TH] Edit [/TH]
close = [TH] Close [/TH]
loading = [TH] Loading… [/TH]
print = [TH] Print [/TH]
back = [TH] Back [/TH]
retry = [TH] Retry [/TH]
search = [TH] Search [/TH]
no-results = [TH] No results found [/TH]
error-occurred = [TH] An error occurred [/TH]

# Audit Log
audit-log-title = [TH] Audit Log [/TH]
audit-log-load-more = [TH] Load More [/TH]
audit-log-loading = [TH] Loading… [/TH]
audit-log-refresh = [TH] Refresh [/TH]
audit-log-retry = [TH] Retry [/TH]
audit-log-filter-all = [TH] All [/TH]
audit-log-filter-success = [TH] Success [/TH]
audit-log-filter-failure = [TH] Failure [/TH]
audit-log-loading-text = [TH] Loading audit log… [/TH]
audit-log-empty-filtered = [TH] No audit entries match the current filters. [/TH]
audit-log-empty-none = [TH] No audit entries recorded yet. Entries appear when sales are completed, voided, or staff actions occur. [/TH]
audit-log-col-date = [TH] Date [/TH]
audit-log-col-action = [TH] Action [/TH]
audit-log-col-target = [TH] Target [/TH]
audit-log-col-user = [TH] User ID [/TH]
audit-log-col-outcome = [TH] Outcome [/TH]
audit-log-col-details = [TH] Details [/TH]
audit-log-count = [TH] { $count } entr{ $count -> [/TH]
  [one] y
  *[other] ies
}

# Update Banner
update-banner-title = [TH] Update available [/TH]
update-banner-new-version = [TH] New version [/TH]
update-banner-install = [TH] Install [/TH]
update-banner-installing = [TH] Installing… [/TH]
update-banner-install-aria = [TH] Download and install update [/TH]
update-banner-installing-aria = [TH] Installing update… [/TH]
update-banner-dismiss-aria = [TH] Dismiss update notification [/TH]

# Toast
toast-dismiss-aria = [TH] Dismiss notification [/TH]
toast-notifications-aria = [TH] Notifications [/TH]

# Modal
modal-close-aria = [TH] Close dialog [/TH]

# Permission Denied
permission-denied-title = [TH] Access Denied [/TH]
permission-denied-desc = [TH] { $action } requires a { $requiredRole } role. [/TH]
permission-denied-current = [TH] You are logged in as { $displayName } ({ $roleName }). [/TH]
permission-denied-go-back = [TH] Go back [/TH]

# Store Switcher
store-switcher-select = [TH] Select Store [/TH]
store-switcher-current-aria = [TH] Current store: { $name }. Click to switch. [/TH]
store-switcher-list-aria = [TH] Stores [/TH]
store-switcher-primary = [TH] · Primary [/TH]

# Gateway Status
gateway-status-online-aria = [TH] { $name } online [/TH]
gateway-status-offline-aria = [TH] { $name } offline [/TH]

# Role Badge
role-badge-logged-in-aria = [TH] Logged in as { $displayName }, { $roleName } [/TH]
role-badge-logout-aria = [TH] Log out { $displayName } [/TH]
role-badge-logout-title = [TH] Log out [/TH]

# Language Selector
language-selector-label = [TH] Language [/TH]
language-selector-select-aria = [TH] Select language [/TH]

# Locale labels
locale-en = [TH] English [/TH]
locale-id = [TH] Bahasa Indonesia [/TH]
locale-th = [TH] ไทย [/TH]

# Accessibility
a11y-skip-to-content = [TH] Skip to main content [/TH]

# Navigation section labels
nav-section-operations = [TH] Operations [/TH]
nav-section-sales = [TH] Sales [/TH]
nav-section-products = [TH] Products [/TH]
nav-section-finance = [TH] Finance [/TH]
nav-section-customers = [TH] Customers [/TH]
nav-section-reports = [TH] Reports [/TH]
nav-section-management = [TH] Management [/TH]
nav-section-inventory = [TH] Inventory [/TH]
nav-section-settings = [TH] Settings [/TH]
nav-section-dev = [TH] Dev [/TH]

nav-pos-terminal = [TH] POS Terminal [/TH]
nav-kds = [TH] KDS [/TH]
nav-products = [TH] Products [/TH]
nav-stock-adjust = [TH] Stock Adjust [/TH]
nav-sales-history = [TH] Sales History [/TH]
nav-dashboard = [TH] Dashboard [/TH]
nav-eod-report = [TH] EOD Report [/TH]
nav-orders = [TH] Orders [/TH]
nav-tax-rates = [TH] Tax Rates [/TH]
nav-exchange-rates = [TH] Exchange Rates [/TH]
nav-categories = [TH] Categories [/TH]
nav-customers = [TH] Customers [/TH]
nav-loyalty = [TH] Loyalty [/TH]
nav-staff = [TH] Staff [/TH]
nav-terminals = [TH] Terminals [/TH]
nav-stores = [TH] Stores [/TH]
nav-features = [TH] Features [/TH]
nav-data = [TH] Data [/TH]
nav-audit-log = [TH] Audit Log [/TH]
nav-offline-queue = [TH] Offline Queue [/TH]
nav-shifts = [TH] Shifts [/TH]
nav-bundles = [TH] Bundles [/TH]
nav-settings = [TH] Settings [/TH]
nav-general = [TH] General [/TH]
nav-dashboard-report = [TH] Dashboard [/TH]
nav-sales-report = [TH] Sales Report [/TH]
nav-inventory-report = [TH] Inventory Report [/TH]
nav-design-system = [TH] Design System [/TH]
nav-tooltip-preview = [TH] Tooltip Preview [/TH]
nav-kiosk = [TH] Kiosk [/TH]
nav-tables = [TH] Tables [/TH]
nav-promotions = [TH] Promotions [/TH]
nav-suppliers = [TH] Suppliers [/TH]
nav-purchase-orders = [TH] Purchase Orders [/TH]
nav-stock-transfers = [TH] Stock Transfers [/TH]
nav-custom-report = [TH] Custom Report [/TH]
nav-pos = [TH] POS [/TH]
nav-stock = [TH] Stock [/TH]
nav-history = [TH] History [/TH]
nav-reports = [TH] Reports [/TH]
nav-section-app = [TH] App [/TH]
nav-sidebar-collapse = [TH] Collapse sidebar [/TH]
nav-sidebar-expand = [TH] Expand sidebar [/TH]
nav-main-aria = [TH] Main navigation [/TH]
nav-tablist-aria = [TH] Navigation tabs [/TH]
nav-switch-workspace = [TH] Switch Workspace [/TH]

# Workspace home
workspace-home-fullscreen-aria = [TH] Toggle fullscreen [/TH]
workspace-home-loading = [TH] Loading workspaces… [/TH]
workspace-home-subtitle = [TH] Select a workspace to start [/TH]
workspace-home-empty = [TH] No workspaces available [/TH]
workspace-home-empty-desc = [TH] You don't have access to any workspaces yet. Contact an administrator. [/TH]
workspace-card-open-aria = [TH] Open { $name } [/TH]
workspace-card-no-access-aria = [TH] { $name } — not available for your role [/TH]
workspace-card-no-access-title = [TH] Your role ({ $role }) cannot access this workspace [/TH]
workspace-card-no-access-badge = [TH] Not available [/TH]
workspace-home-logout = [TH] Logout [/TH]
workspace-home-logout-confirm-title = [TH] Logout? [/TH]
workspace-home-logout-confirm-desc = [TH] You will be returned to the login screen. Any unsaved work will be lost. [/TH]
workspace-home-logout-confirm-cancel = [TH] Cancel [/TH]
workspace-home-logout-confirm-confirm = [TH] Logout [/TH]
workspace-home-shortcut-hint = [TH] Press { $key } to open [/TH]
workspace-home-user-aria = [TH] Logged in as { $name } [/TH]
workspace-home-error-title = [TH] Connection Error [/TH]
workspace-home-error-desc = [TH] Could not load your workspaces. Check your connection and try again. [/TH]
workspace-home-retry = [TH] Try Again [/TH]
workspace-home-retry-btn = [TH] Retry [/TH]
workspace-card-pin-aria = [TH] Pin { $name } to top [/TH]
workspace-card-unpin-aria = [TH] Unpin { $name } [/TH]

# Shell
shell-loading = [TH] Loading… [/TH]

# Status Bar
status-bar-connected = [TH] Backend connected [/TH]
status-bar-disconnected = [TH] Backend disconnected [/TH]
status-bar-checking = [TH] Checking backend connection [/TH]
status-bar-authenticating = [TH] Authenticating... [/TH]
# Sync connection status
status-bar-sync-connected = [TH] Cloud sync connected [/TH]
status-bar-sync-disconnected = [TH] Cloud sync disconnected [/TH]
status-bar-sync-checking = [TH] Checking cloud sync connection… [/TH]
# License status (login screen)
staff-login-license-active = [TH] License active [/TH]
staff-login-license-inactive = [TH] License inactive [/TH]
# P1-3: Tooltip for conflict count badge in StatusBar
statusbar-conflict-count = [TH] { $count } sync conflict(s) resolved [/TH]

# Audit Action Labels
audit-action-sale-void = [TH] Void Sale [/TH]
audit-action-sale-complete = [TH] Complete Sale [/TH]
audit-action-sale-refund = [TH] Refund [/TH]
audit-action-login = [TH] Staff Login [/TH]
audit-action-login-failed = [TH] Login Failed [/TH]
audit-action-user-create = [TH] Staff Created [/TH]
audit-action-user-update = [TH] Staff Updated [/TH]
audit-action-product-create = [TH] Product Created [/TH]
audit-action-product-update = [TH] Product Updated [/TH]
audit-action-product-delete = [TH] Product Deleted [/TH]
audit-action-stock-adjust = [TH] Stock Adjusted [/TH]
audit-action-setting-change = [TH] Setting Changed [/TH]
audit-action-system-backup = [TH] Backup Created [/TH]
audit-action-system-restore = [TH] Restore [/TH]
audit-action-system-export = [TH] Data Export [/TH]
audit-action-system-import = [TH] Data Import [/TH]
audit-log-table-label = [TH] Audit log entries [/TH]
audit-log-search-placeholder = [TH] Search actions, targets, or users… [/TH]
audit-log-search-label = [TH] Search audit log [/TH]
audit-log-filter-label = [TH] Filter by outcome [/TH]

# Auth / License Activation
auth-activate-title = [TH] Activate License [/TH]
auth-activate-subtitle = [TH] Enter your information below [/TH]
auth-email-label = [TH] Email Address [/TH]
auth-email-placeholder = [TH] store@example.com [/TH]
auth-phone-label = [TH] Phone Number [/TH]
auth-phone-placeholder = [TH] 08123456789 [/TH]
auth-license-label = [TH] License Key [/TH]
auth-license-placeholder = [TH] OZ-PRO-XXXX-XXXX-XXXX [/TH]
auth-activate-button = [TH] Activate License [/TH]
auth-activating = [TH] Activating... [/TH]
auth-activation-success = [TH] License activated successfully! [/TH]
auth-activation-failed = [TH] Failed to activate license. [/TH]
auth-activation-error = [TH] An error occurred during activation. [/TH]
auth-validation-required = [TH] License key and Email are required. [/TH]
auth-validation-invalid-email = [TH] Invalid email format. [/TH]
auth-validation-phone-required = [TH] Phone number is required. [/TH]
auth-validation-invalid-phone = [TH] Invalid phone number format. Enter at least 7 digits. [/TH]
auth-paste = [TH] Paste [/TH]
auth-version = [TH] Version { $version } [/TH]
auth-ip-address = [TH] IP Address : { $ip } [/TH]
auth-copyright = [TH] OZ-POS © { $year } All rights reserved. [/TH]
auth-clipboard-error = [TH] Clipboard error: { $message } [/TH]
auth-error-title = [TH] Error [/TH]

## Create Owner PIN (first-run setup)
auth-create-pin-title = [TH] Create Owner PIN [/TH]
auth-create-pin-desc = [TH] Set up the first owner account to manage your POS [/TH]
auth-create-pin-display-name-label = [TH] Display Name [/TH]
auth-create-pin-display-name-placeholder = [TH] Store Owner [/TH]
auth-create-pin-username-label = [TH] Username [/TH]
auth-create-pin-username-placeholder = [TH] owner [/TH]
auth-create-pin-pin-label = [TH] PIN [/TH]
auth-create-pin-pin-placeholder = [TH] At least 4 digits [/TH]
auth-create-pin-confirm-label = [TH] Confirm PIN [/TH]
auth-create-pin-confirm-placeholder = [TH] Re-enter PIN [/TH]
auth-create-pin-creating = [TH] Creating... [/TH]
auth-create-pin-create = [TH] Create Owner Account [/TH]
auth-create-pin-success = [TH] Owner account created successfully! [/TH]
auth-create-pin-error-fields = [TH] All fields are required. [/TH]
auth-create-pin-error-pin-length = [TH] PIN must be at least 4 characters. [/TH]
auth-create-pin-error-pin-mismatch = [TH] PINs do not match. [/TH]
auth-create-pin-error-generic = [TH] An error occurred while creating the owner account. [/TH]

