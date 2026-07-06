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

setup-progress-aria =
    .aria-label = Setup progress

setup-preset-question = What kind of store are you running?
setup-preset-desc = Choose a preset to get started quickly, or customise every feature later.
setup-preset-group-aria =
    .aria-label = Store preset

setup-preset-simple-retail = Simple Retail
setup-preset-simple-retail-desc = Barcode scan, cart, cash/card/QR, staff PIN, receipt printer
setup-preset-restaurant = Restaurant
setup-preset-restaurant-desc = Tables, KDS, split bill, QRIS, shift-based revenue
setup-preset-full-store = Full Store
setup-preset-full-store-desc = Everything except cloud sync and loyalty
setup-preset-custom = Custom
setup-preset-custom-desc = Start from scratch — enable exactly what you need

setup-features-title = { $title }
setup-features-desc = Toggle the features you need. You can change these later.
setup-features-group-aria =
    .aria-label = { $title }
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

# Settings Page
settings-page-title = Settings
settings-loading = Loading settings…
settings-section-store = Store
settings-section-currency = Currency
settings-section-receipt = Receipt
settings-field-store-name = Store name
settings-field-address = Address
settings-field-tax-id = Tax / VAT ID
settings-field-default-currency = Default currency
settings-field-decimal-separator = Decimal separator
settings-field-paper-width = Paper width
settings-field-language = Language
settings-field-footer = Receipt footer
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
settings-saved = Saved!
settings-section-sync = Cloud Sync
settings-sync-server-url = Server URL
settings-sync-api-key = API Key
settings-sync-enabled = Enable Cloud Sync
settings-sync-enabled-aria = Toggle cloud sync
settings-sync-sync-now = Sync Now
settings-sync-syncing = Syncing…
settings-sync-result = Last sync: { $synced } synced, { $failed } failed
settings-store-name-placeholder = OZ-POS Store
settings-address-placeholder = 123 Main Street
settings-tax-id-placeholder = 12-3456789
settings-footer-placeholder = Thank you for shopping!
settings-server-url-placeholder = https://api.example.com
settings-api-key-placeholder = Enter API key
settings-api-key-masked = ••••••••
settings-btn-save-aria =
    .aria-label = { $state ->
        [saved] Saved!
       *[save] Save settings
    }
settings-sync-not-configured = Sync is not configured. Enter a server URL and enable sync.

# Appearance / Brand settings
settings-appearance = Appearance
appearance-primary-colour = Primary Colour
appearance-primary-colour-picker-aria =
    .aria-label = Primary colour picker
appearance-colour-hex-aria =
    .aria-label = Colour hex value
appearance-logo = Store Logo
appearance-logo-alt = Store logo
appearance-choose-logo = Choose Logo
appearance-choose-logo-aria =
    .aria-label = Pick logo file
appearance-store-name = Display Store Name
appearance-store-name-fallback = OZ-POS
appearance-preview = Preview
appearance-preview-btn-label = Primary Button
appearance-preview-btn-outline-label = Secondary
appearance-preview-badge-label = Live
appearance-save-aria =
    .aria-label = Save appearance

# Settings option labels
settings-decimal-separator-dot = 1.00 (dot)
settings-decimal-separator-comma = 1,00 (comma)
settings-decimal-separator-none = 1 (none)
settings-paper-width-standard = 80 mm (standard)
settings-paper-width-narrow = 58 mm (narrow)

# Category Management
category-delete-aria =
    .aria-label = Delete category { $name }
category-delete-dialog-aria =
    .aria-label = Delete category
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
feature-toggle-requires = Requires: { $deps }
feature-toggle-group-aria = { $group } features
feature-toggle-toggle-aria = Toggle { $name }
feature-toggle-dismiss-aria = Dismiss notification

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
settings-footer-placeholder =
    .placeholder = Thank you for shopping!
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
settings-receipt-preview-store-fallback = Toko Anda
settings-receipt-preview-address-fallback = Jl. Contoh No. 123
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
