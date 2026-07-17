# ui/src/locales/settings.ftl — Settings page, setup wizard, sync

# Setup Wizard
setup-logo = OZ-POS
setup-tagline = Point of Sale — Simplified
setup-step-store-type = Store Type
setup-step-payments = Payments
setup-step-products = Products
setup-step-staff = Staff
setup-step-hardware = Hardware
setup-step-business-rules = Business Rules
setup-step-data-cloud = Data & Cloud
setup-step-review = Review
setup-step-aria =
    .aria-label = Step { $number }: { $label }

setup-progress-aria = Setup progress

setup-preset-question = What kind of store are you running?
setup-preset-desc = Choose a preset to get started quickly, or customise every feature later.
setup-preset-group-aria = Store preset

setup-preset-simple-retail = Simple Retail
setup-preset-simple-retail-desc = Barcode scan, cart, cash/card/QR, staff PIN, receipt printer
setup-preset-restaurant = Restaurant
setup-preset-restaurant-desc = Tables, KDS, split bill, QRIS, shift-based revenue
setup-preset-full-store = Full Store
setup-preset-full-store-desc = Everything except cloud sync and loyalty
setup-preset-custom = Custom
setup-preset-custom-desc = Start from scratch — enable exactly what you need

setup-preset-cafe = Cafe / Bakery
setup-preset-cafe-desc = Quick-service with kitchen display, cash+card, discounts
setup-preset-franchise = Franchise
setup-preset-franchise-desc = Multi-store, multi-terminal, restaurant + full admin stack

setup-features-title = { $title }
setup-features-desc = Toggle the features you need. You can change these later.
setup-features-group-aria = { $title }
setup-features-toggle-aria =
    .aria-label = Toggle { $label }

setup-features-section-payments = Payment Methods
setup-features-section-products = Products & Inventory
setup-features-section-staff = Staff Management
setup-features-section-hardware = Hardware & Peripherals
setup-features-section-business-rules = Business Rules
setup-features-section-data-cloud = Data, Reporting & Cloud

# Feature names (short — for review tags)
setup-feature-cash-payment = Cash
setup-feature-card-payment = Card
setup-feature-multi-currency = Multi-Currency
setup-feature-inventory-tracking = Inventory
setup-feature-product-variants = Variants
setup-feature-categories-enabled = Categories
setup-feature-staff-login = Staff Login
setup-feature-staff-roles = Staff Roles
setup-feature-shift-management = Shifts
setup-feature-audit-log = Audit Log
setup-feature-barcode-scanning = Barcode
setup-feature-receipt-printing = Receipts
setup-feature-cash-drawer = Cash Drawer
setup-feature-customer-display = Customer Display
setup-feature-nfc-reader = NFC
setup-feature-discount-engine = Discounts
setup-feature-tax-engine = Tax
setup-feature-loyalty-program = Loyalty
setup-feature-promotions-engine = Promotions
setup-feature-product-bundles = Bundles
setup-feature-reporting = Reports
setup-feature-analytics = Analytics
setup-feature-export-import = Export/Import
setup-feature-cloud-sync = Cloud Sync
setup-feature-multi-store = Multi-Store
setup-feature-multi-terminal = Multi-Terminal
setup-feature-plugin-system = Plugins

# Feature full labels (for toggle rows)
setup-feature-inventory-tracking-label = Inventory Tracking
setup-feature-product-variants-label = Product Variants
setup-feature-shift-management-label = Shift Management
setup-feature-barcode-scanning-label = Barcode Scanner
setup-feature-receipt-printing-label = Receipt Printer
setup-feature-nfc-reader-label = NFC Reader
setup-feature-tax-engine-label = Tax Engine
setup-feature-loyalty-program-label = Loyalty Program
setup-feature-product-bundles-label = Product Bundles
setup-feature-reporting-label = Reporting
setup-feature-export-import-label = Export & Import
setup-feature-plugin-system-label = Plugin System
setup-feature-cash-payment-label = Cash
setup-feature-card-payment-label = Card
setup-feature-multi-currency-label = Multi-Currency
setup-feature-categories-enabled-label = Categories
setup-feature-staff-login-label = Staff Login
setup-feature-staff-roles-label = Staff Roles
setup-feature-audit-log-label = Audit Log
setup-feature-cash-drawer-label = Cash Drawer
setup-feature-customer-display-label = Customer Display
setup-feature-discount-engine-label = Discounts
setup-feature-promotions-engine-label = Promotions
setup-feature-analytics-label = Analytics
setup-feature-cloud-sync-label = Cloud Sync
setup-feature-multi-store-label = Multi-Store
setup-feature-multi-terminal-label = Multi-Terminal

# Feature descriptions
setup-feature-cash-payment-desc = Accept cash payments and track cash drawer
setup-feature-card-payment-desc = Accept debit and credit card payments
setup-feature-multi-currency-desc = Support multiple currencies with exchange rates
setup-feature-inventory-tracking-desc = Track stock levels per product with alerts
setup-feature-product-variants-desc = Size, colour, flavour variants per product
setup-feature-categories-enabled-desc = Group products by category with colour coding
setup-feature-staff-login-desc = PIN or password login for cashiers
setup-feature-staff-roles-desc = Owner, manager, cashier permission levels
setup-feature-shift-management-desc = Open/close shifts with cash reconciliation
setup-feature-audit-log-desc = Immutable log of sensitive actions
setup-feature-barcode-scanning-desc = USB, serial, or Bluetooth barcode scanning
setup-feature-receipt-printing-desc = USB, serial, or network receipt printing
setup-feature-cash-drawer-desc = Automatic cash drawer via printer GPIO
setup-feature-customer-display-desc = Secondary display facing the customer
setup-feature-nfc-reader-desc = Contactless payment and loyalty card reading
setup-feature-discount-engine-desc = Percentage and fixed-amount discounts on items or cart
setup-feature-tax-engine-desc = Tax inclusive/exclusive with configurable rates
setup-feature-loyalty-program-desc = Customer points, tiers, and rewards
setup-feature-promotions-engine-desc = Buy-X-get-Y, time-limited offers, bundles
setup-feature-product-bundles-desc = Sell multiple SKUs together as a single item
setup-feature-reporting-desc = Sales, inventory, and shift reports
setup-feature-analytics-desc = Charts, top products, hourly heatmap, CSV exports
setup-feature-export-import-desc = Encrypted data export and import (.ozpkg)
setup-feature-cloud-sync-desc = Sync data to cloud PostgreSQL with backup
setup-feature-multi-store-desc = Manage multiple store locations
setup-feature-multi-terminal-desc = Multiple POS terminals per store
setup-feature-plugin-system-desc = Third-party plugins and custom drivers

setup-review-title = Review Your Setup
setup-review-desc = Here&rsquo;s a summary of your configuration. You can change anything later.
setup-review-preset = Preset: { $name }
setup-review-enabled = Enabled Features ({ $count })
setup-review-disabled = Disabled Features ({ $count })
setup-review-none = None
setup-review-all-on = Everything on!
setup-review-more = +{ $count } more

setup-default-currency-label = Default Currency

setup-complete-title = All Set!
setup-complete-desc = Your { $preset } POS is configured and ready. You can adjust settings anytime.
setup-launch = Launch OZ-POS
setup-complete-features = { $count } { $count ->
    [one] feature enabled
    *[other] features enabled
}
setup-back = Back
setup-skip = Skip setup
setup-finish = Complete Setup
setup-next = Next

# Live Setup Preview
lsp-title = Feature Preview
lsp-subtitle =
  The sidebar will show { $count } route(s) enabled by your selection
lsp-section-workspaces = Workspaces
lsp-section-nav = Navigation Items
lsp-workspaces-aria = Workspace preview
lsp-nav-aria = Navigation preview
lsp-nav-empty = No navigation items unlocked
lsp-nav-count = { $count } / { $total } items unlocked
lsp-ws-status-active = { $name } — active
lsp-ws-status-inactive = { $name } — inactive

ws-preview-name-restaurant-pos = Restaurant POS
ws-preview-name-store-pos = Store POS
ws-preview-name-kds = Kitchen Display
ws-preview-name-inventory = Inventory
ws-preview-name-admin = Admin

# Settings Page
settings-title = Settings
settings-page-title = Settings
settings-category-business = Business

# ── Sidebar navigation labels ──
settings-nav-general = General
settings-nav-appearance = Appearance
settings-nav-receipt = Receipt
settings-nav-sync = Cloud Sync
settings-nav-about = About
settings-nav-features = Features
settings-nav-data = Data
settings-nav-staff = Staff
settings-nav-terminals = Terminals
settings-nav-stores = Stores
settings-nav-audit = Audit Log
settings-nav-offline = Offline Queue
settings-nav-shifts = Shifts
settings-nav-tax = Tax Rates
settings-nav-license = License
settings-nav-exchange = Exchange Rates
settings-nav-promotions = Promotions
settings-category-operations = Operations
settings-category-system = System
settings-category-management = Management
settings-sidebar-nav-aria = Settings navigation
settings-sidebar-expand-aria = Expand settings sidebar
settings-sidebar-collapse-aria = Collapse settings sidebar
settings-sidebar-collapse-all-aria = Collapse all categories
settings-sidebar-search-clear-aria =
    .aria-label = Clear search
settings-sidebar-no-results = No matching sections
settings-sidebar-clear-results = Clear search
settings-theme-toggle-dark-aria = Switch to dark mode
settings-theme-toggle-light-aria = Switch to light mode
settings-loading = Loading settings…
settings-load-failed = Failed to load settings
settings-load-partial = Some settings could not be loaded. Try again.
settings-section-store = Store
settings-section-currency = Currency
settings-currency-loading = Loading currencies…
settings-section-display = Display
settings-section-receipt = Receipt
settings-field-store-name = Store name
settings-field-address = Address
settings-field-tax-id = Tax / VAT ID
settings-field-default-currency = Default currency
settings-field-decimal-separator = Decimal separator
settings-field-paper-width = Paper width
settings-field-footer = Receipt footer

# ── Display sub-section fields ──
settings-field-card-size = Menu Card Size
settings-field-font-size = Font Size
settings-card-size-decrease-aria =
    .aria-label = Decrease card size
settings-card-size-increase-aria =
    .aria-label = Increase card size
settings-font-size-decrease-aria =
    .aria-label = Decrease font size
settings-font-size-increase-aria =
    .aria-label = Increase font size
settings-field-font-smoothing = Font Smoothing
settings-toggle-show-currency = Show currency symbol on amounts
settings-toggle-show-currency-aria =
    .aria-label = Show currency symbol on amounts
settings-toggle-show-tax = Show tax line on receipts
settings-toggle-show-tax-aria =
    .aria-label = Show tax line on receipts
settings-toggle-show-table-number = Show table number on cart and receipts
settings-toggle-show-table-number-aria =
    .aria-label = Show table number on cart and receipts
settings-btn-save = Save
settings-btn-revert = Revert

settings-btn-revert-aria =
    .aria-label = Revert settings to last saved state

settings-saved = Saved!
settings-section-sync = Cloud Sync
settings-sync-server-url = Server URL
settings-sync-api-key = API Key
settings-sync-enabled = Enable Cloud Sync
settings-sync-enabled-aria = Toggle cloud sync
settings-sync-sync-now = Sync Now
settings-sync-syncing = Syncing…
settings-sync-test-connection = Test Connection
settings-sync-testing = Testing…
settings-sync-test-failed = Connection test failed
settings-sync-token-request-failed = Token request failed — check server URL
settings-sync-request-token = Request Token
settings-sync-requesting = Requesting…
settings-sync-error = Sync failed

# ── Token expiry badge ──────────────────────────────────
settings-sync-expiry-expired = Expired
settings-sync-expiry-in-days = { $count ->
    [one] Expires in 1 day
   *[other] Expires in { $count } days
}
settings-sync-expiry-in-hours = { $count ->
    [one] Expires in 1 hour
   *[other] Expires in { $count } hours
}
settings-sync-expiry-in-minutes = { $count ->
    [one] Expires in 1 minute
   *[other] Expires in { $count } minutes
}
settings-sync-expiry-less-than-minute = Expires in less than a minute
settings-sync-expiry-fallback = Expires { $iso }
settings-sync-result = Last sync: { $synced } synced, { $failed } failed
settings-sync-success = Sync complete: { $synced } synced, { $failed } failed
settings-sync-nothing = Nothing to sync — all caught up
settings-store-name-placeholder = OZ-POS Store
settings-address-placeholder = 123 Main Street
settings-tax-id-placeholder = 12-3456789
settings-footer-placeholder = Thank you for shopping!
settings-server-url-placeholder = https://api.example.com
settings-api-key-placeholder = Enter API key
settings-api-key-masked = ••••••••
settings-api-key-show-aria = Show API key
settings-api-key-hide-aria = Hide API key
settings-btn-save-aria =
    .aria-label = { $state ->
        [saved] Saved!
       *[save] Save settings
    }
settings-save-error = Failed to save settings. Please try again.
settings-save-partial = Some settings could not be saved. Try again.
settings-retry = Retry
settings-sync-not-configured = Sync is not configured. Enter a server URL and enable sync.
settings-sync-status-idle = Ready
settings-sync-status-ok = Connected
settings-sync-status-error = Connection error
settings-sync-pending-count = { $count } pending
settings-sync-pull = Pull from Server
settings-sync-pulling = Pulling…
settings-sync-pull-empty = Server returned empty snapshot — nothing to pull
settings-sync-pull-result = Last pull: { $products } products, { $tax_rates } tax rates, { $users } users
settings-font-smoothing-antialiased = Antialiased (crisp)
settings-font-smoothing-subpixel = Subpixel (smooth)

# ── System & License ──
settings-system-license-header = System & License Ownership
settings-software-edition = Software Edition
settings-license-type = License Type
settings-copyright-notice = Copyright Notice
settings-commercial-contact = Commercial Contact
settings-app-version = OZ-POS Enterprise v{ $version }

# ── License Info Section ──
settings-section-license = License
settings-license-tier = Tier
settings-license-status-label = Status
settings-license-expires = Expires
settings-license-grace = Grace Period Until
settings-license-max-stores = Max Stores
settings-license-max-pos = Max POS Instances
settings-license-tenant-id = Tenant ID
settings-license-allowed-types = Allowed Workspace Types
settings-license-allowed-types-all = All
settings-license-not-activated = No license activated. Activate a license to see details here.
settings-license-check-server = Check Server Status
settings-license-server-tier = Server Tier
settings-license-server-active = Server Active
settings-license-server-expires = Server Expires
settings-license-server-results = License Check Results
settings-license-type-value = Proprietary
settings-license-unlimited = Unlimited
settings-license-yes = Yes
settings-license-no = No
settings-license-status-active = Active
settings-license-tier-free = Free
settings-license-tier-pro = Pro
settings-license-tier-premium = Premium
settings-license-tier-enterprise = Enterprise
settings-license-ws-retail = Retail
settings-license-ws-restaurant = Restaurant
settings-license-ws-cafe = Café
settings-license-ws-kiosk = Kiosk
settings-license-ws-franchise = Franchise
settings-license-ws-warehouse = Warehouse
settings-license-server-status-retrieved = Server license status retrieved.
settings-license-server-check-failed = Server check failed
settings-license-server-status = Server Status
settings-license-live-online = Live
settings-license-live-offline = Offline
settings-license-live-inactive = Inactive
settings-license-live-checking = Checking…
settings-license-last-checked = Last checked: { $when }
settings-license-just-now = just now
settings-license-seconds-ago = { $seconds }s ago
settings-license-minutes-ago = { $minutes }m ago
settings-license-refresh = Refresh
settings-license-refresh-aria = Refresh license status
settings-license-poll-offline = Server unreachable
settings-license-load-failed = Failed to load license info
settings-copyright-notice-value = OZ-POS © 2025–2026 OZ Systems. All rights reserved.

# Appearance / Brand settings
settings-appearance = Appearance
appearance-primary-colour = Primary Colour
appearance-primary-colour-picker-aria =
    .aria-label = Primary colour picker
appearance-colour-hex-aria =
    .aria-label = Colour hex value
appearance-reset-colour-aria =
    .aria-label = Reset colour to default
appearance-reset-colour = Reset to default
appearance-logo = Store Logo
appearance-logo-alt = Store logo
appearance-choose-logo = Choose Logo
appearance-choose-logo-aria =
    .aria-label = Pick logo file
appearance-store-name = Display Store Name
appearance-interface-zoom = Interface Zoom
appearance-zoom-auto = Automatic (Scale with screen)
appearance-zoom-100 = 100% (Default)
appearance-zoom-125 = 125%
appearance-zoom-150 = 150%
appearance-zoom-200 = 200%
appearance-branding = Branding
appearance-interface = Interface
appearance-preview-heading = Preview
appearance-store-name-fallback = OZ-POS
appearance-hw-accel = Hardware Acceleration
appearance-hw-accel-aria =
    .aria-label = Toggle hardware acceleration
appearance-hw-accel-on = On
appearance-hw-accel-off = Off
appearance-hw-accel-hint = Disable if UI animations feel janky on low-end devices. Restart the app for the change to take full effect.
appearance-preview = Preview
appearance-preview-btn-label = Primary Button
appearance-preview-btn-outline-label = Secondary
appearance-preview-badge-label = Live
appearance-reset-all-aria =
    .aria-label = Reset all to defaults
appearance-reset-all = Reset all to defaults
appearance-reset-all-confirm = Reset all appearance settings to their defaults? This cannot be undone.
appearance-reset-all-success = Appearance settings reset to defaults
appearance-reset-all-failed = Failed to reset appearance settings
appearance-save-aria =
    .aria-label = Save appearance
appearance-save-success = Appearance settings saved
appearance-save-failed = Failed to save appearance settings

# Settings option labels
settings-decimal-separator-dot = 1.00 (dot)
settings-decimal-separator-comma = 1,00 (comma)
settings-decimal-separator-none = 1 (none)
settings-paper-width-standard = 80 mm (standard)
settings-paper-width-narrow = 58 mm (narrow)

# Category Management
category-delete-aria =
    .aria-label = Delete category { $name }
category-delete-dialog-aria = Delete category
category-colour-picker-aria =
    .aria-label = Pick a colour
category-colour-swatch-aria =
    .aria-label = Select colour { $colour }
category-name-fallback = Category Name

# Data Management Screen
data-mgmt-title = Data Management
data-mgmt-tabs-aria = Data management actions
data-mgmt-tab-export = Export
data-mgmt-tab-import = Import
data-mgmt-tab-backup = Backup

# Export wizard
data-mgmt-export-wizard-aria = Export wizard
data-mgmt-export-title = Select data to export
data-mgmt-export-types-aria = Data types to export
data-mgmt-export-select-all = Select all / none
data-mgmt-type-products = Products
data-mgmt-type-products-desc = SKU, name, price, barcode, stock
data-mgmt-type-categories = Categories
data-mgmt-type-categories-desc = Category id, name, colour
data-mgmt-type-sales = Sales
data-mgmt-type-sales-desc = Sale header, line items, payments
data-mgmt-type-customers = Customers
data-mgmt-type-customers-desc = Name, email, phone, loyalty points
data-mgmt-type-users = Users
data-mgmt-type-users-desc = Usernames, display names, roles (no passwords)
data-mgmt-type-settings = Settings
data-mgmt-type-settings-desc = Store config, receipts, feature flags
data-mgmt-export-date-from = From
data-mgmt-export-date-to = To
data-mgmt-export-next = Next: Encryption
data-mgmt-export-exporting = Exporting…
data-mgmt-export-complete = Export complete
data-mgmt-export-done-text = Data exported to:
data-mgmt-export-selected-types = Selected types:
data-mgmt-export-new-export = New export

# Encryption step
data-mgmt-encrypt-title = Set encryption password
data-mgmt-encrypt-desc = The export file will be encrypted with AES-256-GCM. Choose a strong password — you will need it to import the data later.
data-mgmt-encrypt-password = Password
data-mgmt-encrypt-password-placeholder = At least 8 characters
data-mgmt-encrypt-confirm = Confirm password
data-mgmt-encrypt-confirm-placeholder = Re-enter password
data-mgmt-encrypt-back = Back
data-mgmt-encrypt-export = Export

# Import wizard
data-mgmt-import-wizard-aria = Import wizard
data-mgmt-import-title = Select a backup file
data-mgmt-import-desc = Choose an encrypted .ozpkg file to import. The file must have been created by OZ-POS export.
data-mgmt-import-drop-text = Drag & drop a .ozpkg file here, or
data-mgmt-import-browse = Browse files…
data-mgmt-import-preview-title = Preview import
data-mgmt-import-meta-file = File
data-mgmt-import-meta-not-selected = Not selected
data-mgmt-import-meta-store = Store
data-mgmt-import-meta-version = Version
data-mgmt-import-meta-created = Created
data-mgmt-import-meta-contains = Contains
data-mgmt-import-password = Decryption password
data-mgmt-import-password-placeholder = Enter the export password
data-mgmt-import-cancel = Cancel
data-mgmt-analyse-file = Analyse file
data-mgmt-import-start = Start import
data-mgmt-import-analysing = Analysing file…
data-mgmt-import-dry-run-complete = Dry-run complete — importing…
data-mgmt-import-dry-run-title = Changes to be applied
data-mgmt-import-dry-run-added = New items
data-mgmt-import-dry-run-updated = Updated
data-mgmt-import-dry-run-skipped = Skipped
data-mgmt-import-complete = Import complete
data-mgmt-import-done-text = All data has been imported successfully.
data-mgmt-import-done-summary = { $added } items added, { $updated } updated, { $skipped } skipped.
data-mgmt-import-new-import = New import

# Backup section
data-mgmt-backup-status-aria = Backup status
data-mgmt-backup-title = Database backup
data-mgmt-backup-desc = Create an online snapshot of the current database. The backup runs in the background and does not interrupt POS operations.
data-mgmt-backup-label-last = Last backup
data-mgmt-backup-never = Never
data-mgmt-backup-label-size = Size
data-mgmt-backup-create = Create backup now
data-mgmt-backup-backing-up = Backing up…

# Toast notifications
data-mgmt-toast-backup-success = Backup created successfully
data-mgmt-export-complete-aria = Export complete
data-mgmt-import-complete-aria = Import complete
data-mgmt-toast-backup-fail = Backup failed
data-mgmt-toast-export-select-type = Select at least one data type to export
data-mgmt-toast-export-password-length = Password must be at least 8 characters
data-mgmt-toast-export-password-match = Passwords do not match
data-mgmt-toast-export-success = Export complete
data-mgmt-toast-export-fail = Export failed
data-mgmt-toast-import-enter-password = Enter the export password
data-mgmt-toast-import-no-file = No file selected
data-mgmt-toast-import-success = Import complete
data-mgmt-toast-import-fail = Import failed
data-mgmt-toast-file-picker-fail = Failed to open file picker
data-mgmt-toast-backup-status-fail = Failed to load backup status

# Aria labels
data-mgmt-dismiss-aria = Dismiss notification

# Feature Toggles
feature-toggle-title = Feature Toggles
feature-toggle-subtitle = { $enabled } / { $total } enabled
feature-toggle-loading = Loading features…
feature-toggle-group-core = Core
feature-toggle-group-payments = Payments
feature-toggle-group-products = Products
feature-toggle-group-staff = Staff
feature-toggle-group-hardware = Hardware
feature-toggle-group-business-rules = Business Rules
feature-toggle-group-restaurant = Restaurant
feature-toggle-group-scaling = Scaling
feature-toggle-group-reporting = Reporting
feature-toggle-group-advanced = Advanced
feature-toggle-error-load = Failed to load features
feature-toggle-error-toggle = Failed to toggle feature
feature-toggle-enabled = Feature enabled
feature-toggle-disabled = Feature disabled
feature-toggle-auto-enabled = Auto-enabled dependencies: { $list }
feature-toggle-retry = Retry
feature-toggle-empty = No features found.
feature-toggle-empty-search = No features match your search.
feature-toggle-search-placeholder =
    .placeholder = Search features…
feature-toggle-search-aria = Search features
feature-toggle-search-clear-aria = Clear search
feature-toggle-bulk-enable = Enable All
feature-toggle-bulk-disable = Disable All
feature-toggle-bulk-enable-aria = Enable all { $group } features
feature-toggle-bulk-disable-aria = Disable all { $group } features
feature-toggle-bulk-enabled = All { $group } features enabled
feature-toggle-bulk-disabled = All { $group } features disabled
feature-toggle-requires = Requires: { $deps }
feature-toggle-group-aria = { $group } features
feature-toggle-toggle-aria = Toggle { $name }
feature-toggle-dismiss-aria = Dismiss notification

# ── Data Management ──
data-mgmt-password-show-aria = Show password
data-mgmt-password-hide-aria = Hide password

# ── Settings screen ──
settings-general-tab = General
settings-receipt-tab = Receipt
settings-printer-tab = Printer
settings-scanner-tab = Scanner
settings-credit-tab = Credit
settings-system-tab = System
settings-header-options = Options
settings-general-heading = General Settings
settings-branch-label = Branch
settings-branch-placeholder =
    .placeholder = Main branch
settings-tax-id-label = Tax ID
settings-currency-options-label = Currency options
settings-receipt-heading = Receipt Settings
settings-show-currency-label = Show currency symbol
settings-show-tax-label = Show tax line
settings-show-table-label = Show table number
settings-decimal-sep-label = Decimal separator
settings-paper-width-label = Paper width
settings-footer-label = Footer text
settings-margins-heading = Paper Margins (mm)
settings-margin-top = Top
settings-margin-bottom = Bottom
settings-margin-left = Left
settings-margin-right = Right
settings-click-preview = Click to preview
settings-printer-heading = Receipt Printer
settings-connection-label = Connection
settings-printer-connection-auto = Auto-detect
settings-printer-connection-usb = USB
settings-printer-connection-serial = Serial (COM)
settings-printer-connection-network = Network (TCP/IP)
settings-device-path-label = Device path
settings-device-path-placeholder =
    .placeholder = /dev/usb/lp0 or COM1
settings-printer-paper-size-label = Paper size
settings-printer-info = Printer configuration is stored on this terminal. Changes apply after restart.
settings-scanner-heading = Barcode Scanner
settings-scanner-none = No scanners detected. Connect a scanner and restart.
settings-scanner-device-label = Scanner device
settings-auto-add-label = Auto-add product on scan
settings-input-mode-label = Input mode
settings-input-mode-auto = Auto-detect
settings-input-mode-keyboard = Keyboard wedge
settings-input-mode-serial = Serial/COM
settings-credit-heading = Credit Settings
settings-enable-credit-label = Enable credit sales
settings-reminder-interval-label = Reminder interval (hours)
settings-reminder-interval-hint = How often the credit reminder badge appears on the POS screen
settings-max-credit-label = Max credit limit (Rp)
settings-max-credit-hint = Maximum outstanding balance allowed per customer (0 = unlimited)
settings-credit-status-enabled = enabled
settings-credit-status-disabled = disabled
settings-credit-status-max = Max limit: Rp { $amount }
settings-credit-status-no-limit = No limit set.
settings-credit-status-label = Credit sales are currently { $status }.
settings-app-version-label = App version
settings-cashier-label = Cashier
settings-terminal-label = Terminal
settings-theme-label = Theme
settings-theme-light = Light
settings-theme-dark = Dark
settings-system-heading = System
settings-auto-lock-label = Auto-lock after (minutes)
settings-auto-lock-hint = Lock the session after inactivity (1–120 min)
settings-saving-btn = Saving…
settings-toast-failed-tax-rates = Failed to load tax rates
settings-toast-failed-store-settings = Failed to load store settings
settings-toast-failed-receipt-settings = Failed to load receipt settings
settings-toast-failed-credit-settings = Failed to load credit settings
settings-toast-failed-hardware-settings = Failed to load hardware settings
settings-toast-failed-scanners = Failed to load scanners
settings-toast-saved = Settings saved
settings-toast-failed-save = Failed to save settings

# Settings receipt preview
settings-receipt-preview-store-fallback = Your Store
settings-receipt-preview-address-fallback = 123 Main Street
settings-receipt-preview-cashier = Cashier:
settings-receipt-preview-col-item = Item
settings-receipt-preview-col-qty = Qty
settings-receipt-preview-col-price = Price
settings-receipt-preview-subtotal = Subtotal
settings-receipt-preview-tax = PPN
settings-receipt-preview-total = TOTAL

# Settings decimal separator options
settings-decimal-sep-dot = Dot (.)
settings-decimal-sep-comma = Comma (,)
settings-decimal-sep-none = None

# Settings paper width options
settings-paper-narrow = 58 mm (thermal)
settings-paper-standard = 80 mm (thermal)
settings-paper-a4 = A4 (210 × 297 mm)
settings-paper-letter = Letter (8.5 × 11 in)
settings-paper-9x11 = 9.5 × 11 in (3-ply NCR / continuous)
settings-paper-9x5 = 9.5 × 5.5 in (half-sheet continuous)

# Payments tab
settings-payments-tab = Payments
settings-payments-heading = Payment Gateways
settings-payments-no-gateways = No payment gateways configured
settings-stripe-api-key = Stripe API Key
settings-stripe-key-placeholder = sk_live_...
settings-stripe-key-hint = Enter your Stripe secret key to enable card payments
settings-square-api-key = Square API Key
settings-square-key-placeholder = sq0atp-...
settings-square-key-hint = Enter your Square access token to enable card payments
settings-midtrans-key = Midtrans Server Key
settings-midtrans-key-placeholder = Mid-server-...
settings-midtrans-key-hint = Enter your Midtrans server key for QRIS payments

# Tender presets
settings-tender-presets-heading = Quick Cash Tender Buttons
settings-tender-presets-desc = Customize the quick tender buttons shown on the cash payment screen. Values are in rupiah (e.g., 50000 = Rp 50,000).
settings-tender-preset-label = Preset { $n }
settings-tender-preset-aria = Preset { $n } amount
settings-tender-preset-remove-aria = Remove preset { $n }
settings-tender-preset-add = Add preset

# Sound & language
settings-sound-label = Sound Effects
settings-language-label = Language

# Quick links
settings-quick-links-heading = More Configuration
settings-quick-links-note = Tax rates and feature toggles can be configured from the main Settings page, accessible via the sidebar.

# Customer-facing display
settings-display-heading = Customer-Facing Display
settings-display-none = No pole displays detected. Connect a customer-facing display to enable this feature.
settings-display-count = { $count } display(s) connected
settings-display-test-label = Test Message
settings-display-test-placeholder = Welcome to our store!
settings-display-test-btn = Show
settings-display-test-sent = Message sent to display
settings-display-test-failed = Failed to send to display
settings-display-info = The customer-facing display shows item names and totals as they are scanned during a sale.
settings-toast-failed-displays = Failed to load displays

# New tab labels
settings-appearance-tab = Appearance
settings-features-tab = Features
settings-data-tab = Data
settings-sync-tab = Sync

# Section headings (when a sub-screen doesn't render its own)
settings-appearance-heading = Appearance
settings-features-heading = Feature Toggles
settings-data-heading = Data Management
settings-sync-heading = Cloud Sync

# Sync tab
settings-sync-enabled-label = Enable cloud sync
settings-sync-server-label = Server URL
settings-sync-server-placeholder = https://sync.oz-pos.example.com
settings-sync-server-hint = The endpoint that receives your encrypted backup snapshots
settings-sync-token-label = Authentication Token
settings-sync-token-placeholder = paste sync token here
settings-sync-token-hint = Stored securely in the database — never in localStorage
settings-sync-interval-label = Auto-sync interval (minutes)
settings-sync-interval-hint = Set to 0 to disable automatic sync
settings-sync-now-btn = Sync now
settings-sync-testing-btn = Testing…
settings-sync-test-connection-btn = Test connection
settings-sync-status-online = Online
settings-sync-status-offline = Offline
settings-sync-status-never = Never synced
settings-sync-last = Last sync
settings-sync-last-at = Last sync: { $when }
settings-sync-pending = Pending changes
settings-sync-toast-success = Sync completed successfully
settings-sync-toast-fail = Sync failed — check server URL and token
settings-sync-toast-test-success = Connection test passed
settings-sync-toast-test-fail = Could not reach server
settings-sync-confirm-overwrite = Overwrite local data with the server snapshot?
settings-sync-force-pull-btn = Pull from server
settings-sync-pulling-btn = Pulling…
settings-sync-pull-toast-success = Pulled { $products } products, { $tax_rates } tax rates, { $users } users from server
settings-sync-pull-toast-empty = Server snapshot was empty — nothing to pull
settings-sync-pull-toast-fail = Pull failed — check server URL and token
settings-toast-failed-sync = Failed to load sync settings
settings-field-language = Language

# ── Field validation ──
settings-store-name-required = Store name is required
settings-tax-id-pattern-error = Only letters, numbers, dashes, dots, and slashes allowed
settings-tax-id-pattern-hint = Letters, numbers, dashes, dots, and slashes only, max 20 characters

# ── Updates ──
settings-updates-heading = Updates
settings-current-version = Current Version
settings-check-for-updates = Check for Updates
settings-checking-for-updates = Checking…
settings-up-to-date = ✓ You're up to date
settings-update-available = { $version } is available
settings-install-update = Install Now
settings-installing-update = Installing…
settings-update-status-label = Status
settings-update-not-checked = Not checked
settings-update-check-error = Update check failed
settings-update-retry = Retry
