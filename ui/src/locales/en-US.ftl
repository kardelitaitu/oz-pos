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
