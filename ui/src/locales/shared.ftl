# ui/src/locales/shared.ftl — Shared UI strings used across features
#
# IDs are `feature-element[-qualifier]`.

# Design system showcase
ds-title = Design System
theme-toggle-label = Toggle theme

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

# Error state
error-state-title = Something went wrong
error-state-desc = An unexpected error occurred. Please try again.
error-state-retry = Retry

# Navigation
nav-inventory = Inventory

# Audit Log
audit-log-title = Audit Log
audit-log-load-more = Load More
audit-log-loading = Loading…
audit-log-refresh = Refresh
audit-log-retry = Retry
audit-log-search-placeholder = Search actions, targets, or users…
audit-log-search-aria = Search audit log
audit-log-outcome-aria = Filter by outcome
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
