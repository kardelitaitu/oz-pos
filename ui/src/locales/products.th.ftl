# ui/src/locales/products.ftl — Product management, lookup, variants

# ── Restaurant Menu ──────────────────────────────────────────────────
restaurant-menu-search-placeholder =
    .placeholder = [TH] Search Menu [/TH]
restaurant-menu-hamburger-aria =
    .aria-label = [TH] Menu [/TH]
restaurant-menu-back-aria =
    .aria-label = [TH] Back to workspaces [/TH]
restaurant-size-decrease-aria =
    .aria-label = [TH] Decrease size [/TH]
restaurant-size-increase-aria =
    .aria-label = [TH] Increase size [/TH]
restaurant-font-size-decrease-aria =
    .aria-label = [TH] Decrease font size [/TH]
restaurant-font-size-increase-aria =
    .aria-label = [TH] Increase font size [/TH]
restaurant-font-size-label = [TH] Font Size [/TH]
restaurant-theme-light = [TH] Light Mode [/TH]
restaurant-theme-dark = [TH] Dark Mode [/TH]
restaurant-lock-terminal = [TH] Lock Terminal [/TH]
restaurant-toggle-fullscreen = [TH] Toggle Fullscreen [/TH]
restaurant-clear-color-aria =
    .aria-label = [TH] Clear color [/TH]
restaurant-categories-aria =
    .aria-label = [TH] Menu categories [/TH]
restaurant-menu-loading = [TH] Loading menu… [/TH]
restaurant-menu-empty = [TH] Menu is empty [/TH]
restaurant-size-label = [TH] Size [/TH]
restaurant-sort-label = [TH] Sort [/TH]
restaurant-card-add = [TH] Add [/TH]
restaurant-context-color-label = [TH] Color [/TH]

# Product Lookup
product-lookup-title = [TH] Products [/TH]
product-lookup-dev-fallback = [TH] Using sample data (IPC unavailable) [/TH]
product-lookup-search-placeholder =
    .placeholder = [TH] Search products… [/TH]
product-lookup-search-aria =
    .aria-label = [TH] Search for products by name, SKU, or barcode [/TH]
product-lookup-barcode-placeholder =
    .placeholder = [TH] Scan barcode… [/TH]
product-lookup-barcode-aria =
    .aria-label = [TH] Enter or scan a barcode [/TH]
product-lookup-barcode-scan = [TH] Scan [/TH]
product-lookup-scan-btn-aria =
    .aria-label = [TH] Submit the entered barcode [/TH]
product-lookup-no-results = [TH] No products found [/TH]
product-lookup-loading = [TH] Loading products… [/TH]
product-lookup-add = [TH] Add to cart [/TH]
product-lookup-in-stock = [TH] In stock [/TH]
product-lookup-out-of-stock = [TH] Out of stock [/TH]
product-lookup-all-categories = [TH] All Categories [/TH]
product-lookup-categories-aria = [TH] Filter by category [/TH]
product-lookup-grid-aria = [TH] Product search results [/TH]
product-lookup-card-aria =
    .aria-label = [TH] { $name } — { $price }. SKU: { $sku }. { $stock } [/TH]
product-lookup-bundle-added = [TH] Bundle "{ $name }" added — { $count } items [/TH]
product-lookup-no-match = [TH] No product or bundle matches this barcode [/TH]
product-lookup-uncategorised = [TH] Uncategorised [/TH]
product-lookup-error-load = [TH] Failed to load products [/TH]

# Product Management
product-mgmt-title = [TH] Products [/TH]
product-mgmt-add = [TH] Add Product [/TH]
product-mgmt-loading = [TH] Loading products… [/TH]
product-mgmt-empty = [TH] No products yet. [/TH]
product-mgmt-empty-cta = [TH] Add your first product [/TH]
product-mgmt-col-sku = [TH] SKU [/TH]
product-mgmt-col-name = [TH] Name [/TH]
product-mgmt-col-category = [TH] Category [/TH]
product-mgmt-col-price = [TH] Price [/TH]
product-mgmt-col-barcode = [TH] Barcode [/TH]
product-mgmt-col-stock = [TH] Stock [/TH]
product-mgmt-stock-in = [TH] In stock [/TH]
product-mgmt-stock-out = [TH] Out of stock [/TH]
product-mgmt-edit = [TH] Edit [/TH]
product-mgmt-edit-aria = [TH] Edit { $name } [/TH]
product-mgmt-delete = [TH] Delete [/TH]
product-mgmt-delete-aria = [TH] Delete { $name } [/TH]
product-mgmt-deleting =
    { $count ->
        [one] Deleting…
       *[other] …
    }
product-mgmt-modal-add-title = [TH] Add Product [/TH]
product-mgmt-modal-edit-title = [TH] Edit Product [/TH]
product-mgmt-modal-close = [TH] Close [/TH]
product-mgmt-field-sku = [TH] SKU [/TH]
product-mgmt-field-sku-required = [TH] SKU * [/TH]
product-mgmt-field-name = [TH] Name [/TH]
product-mgmt-field-name-required = [TH] Name * [/TH]
product-mgmt-field-price = [TH] Price (minor units) [/TH]
product-mgmt-field-currency = [TH] Currency [/TH]
product-mgmt-field-category = [TH] Category [/TH]
product-mgmt-field-barcode = [TH] Barcode [/TH]
product-mgmt-field-tax-rates = [TH] Tax Rates [/TH]
product-mgmt-field-stock = [TH] Initial stock [/TH]
product-mgmt-btn-cancel = [TH] Cancel [/TH]
product-mgmt-btn-create = [TH] Create [/TH]
product-mgmt-btn-update = [TH] Update [/TH]
product-mgmt-table-aria = [TH] Product catalog [/TH]
product-mgmt-actions-aria =
    .aria-label = [TH] Actions [/TH]
product-mgmt-variants = [TH] Variants [/TH]
product-mgmt-variants-aria =
    .aria-label = [TH] Variants for { $name } [/TH]
product-mgmt-modal-aria =
    .aria-label = [TH] { $mode -> [/TH]
        [add] Add product
       *[edit] Edit product
    }
product-mgmt-modal-close-aria =
    .aria-label = [TH] Close [/TH]
product-mgmt-sku-placeholder =
    .placeholder = [TH] e.g. LATTE [/TH]
product-mgmt-name-placeholder =
    .placeholder = [TH] e.g. Caffè Latte [/TH]
product-mgmt-price-placeholder =
    .placeholder = [TH] 450 [/TH]
product-mgmt-barcode-placeholder =
    .placeholder = [TH] 4901234567890 [/TH]
product-mgmt-stock-placeholder =
    .placeholder = [TH] 0 [/TH]
product-mgmt-no-category = [TH] — No category — [/TH]
product-mgmt-col-type = [TH] Type [/TH]
product-mgmt-field-type = [TH] Type [/TH]
product-mgmt-alerts-title = [TH] Stock Alerts [/TH]
product-mgmt-alert-close = [TH] Close [/TH]

# Product Variants
variant-mgmt-title = [TH] Variants — { $product } [/TH]
variant-mgmt-loading = [TH] Loading variants… [/TH]
variant-mgmt-empty = [TH] No variants yet. [/TH]
variant-mgmt-empty-cta = [TH] Add a variant [/TH]
variant-mgmt-add = [TH] Add Variant [/TH]
variant-mgmt-col-name = [TH] Name [/TH]
variant-mgmt-col-sku = [TH] SKU [/TH]
variant-mgmt-col-price = [TH] Price [/TH]
variant-mgmt-col-barcode = [TH] Barcode [/TH]
variant-mgmt-col-status = [TH] Status [/TH]
variant-mgmt-price-parent = [TH] Uses parent price [/TH]
variant-mgmt-status-active = [TH] Active [/TH]
variant-mgmt-status-inactive = [TH] Inactive [/TH]
variant-mgmt-edit = [TH] Edit [/TH]
variant-mgmt-edit-aria = [TH] Edit { $name } [/TH]
variant-mgmt-delete = [TH] Delete [/TH]
variant-mgmt-delete-aria = [TH] Delete { $name } [/TH]
variant-mgmt-delete-confirm-title = [TH] Delete Variant [/TH]
variant-mgmt-delete-confirm-body = [TH] Are you sure you want to delete variant "{ $name }" ({ $sku })? This action cannot be undone. [/TH]
variant-mgmt-delete-confirm-cancel = [TH] Cancel [/TH]
variant-mgmt-delete-confirm-confirm = [TH] Delete [/TH]
variant-mgmt-modal-add-title = [TH] Add Variant [/TH]
variant-mgmt-modal-edit-title = [TH] Edit Variant [/TH]
variant-mgmt-modal-close = [TH] Close [/TH]
variant-mgmt-close = [TH] Close [/TH]
variant-mgmt-close-aria =
    .aria-label = [TH] Close [/TH]
variant-mgmt-field-name-required = [TH] Name * [/TH]
variant-mgmt-field-sku-required = [TH] SKU * [/TH]
variant-mgmt-field-price = [TH] Price (minor units) [/TH]
variant-mgmt-field-currency = [TH] Currency [/TH]
variant-mgmt-field-barcode = [TH] Barcode [/TH]
variant-mgmt-field-sort-order = [TH] Sort order [/TH]
variant-mgmt-field-active = [TH] Active [/TH]
variant-mgmt-btn-cancel = [TH] Cancel [/TH]
variant-mgmt-btn-create = [TH] Create [/TH]
variant-mgmt-btn-update = [TH] Update [/TH]
variant-mgmt-overlay-aria = [TH] Variants for { $name } [/TH]
variant-mgmt-dialog-aria = [TH] { $mode -> [/TH]
    [add] Add variant
   *[edit] Edit variant
}
variant-mgmt-table-aria = [TH] Product variants [/TH]
variant-mgmt-actions-aria =
    .aria-label = [TH] Actions [/TH]
variant-mgmt-name-placeholder =
    .placeholder = [TH] e.g. Large [/TH]
variant-mgmt-sku-placeholder =
    .placeholder = [TH] e.g. TEA-LARGE [/TH]
variant-mgmt-price-placeholder =
    .placeholder = [TH] 450 [/TH]
variant-mgmt-currency-placeholder =
    .placeholder = [TH] USD [/TH]
variant-mgmt-barcode-placeholder =
    .placeholder = [TH] 4901234567890 [/TH]
variant-mgmt-sort-placeholder =
    .placeholder = [TH] 0 [/TH]
variant-mgmt-delete-confirm-aria =
    .aria-label = [TH] Delete confirmation [/TH]
variant-mgmt-error-load = [TH] Failed to load variants [/TH]
variant-mgmt-error-save = [TH] Failed to save variant [/TH]
variant-mgmt-error-delete = [TH] Failed to delete variant [/TH]

# Category Management
categories-title = [TH] Categories [/TH]
categories-loading = [TH] Loading categories… [/TH]
categories-no-categories = [TH] No categories yet [/TH]
categories-empty-desc = [TH] Get started by creating your first category [/TH]
categories-add-first = [TH] Add your first category [/TH]
categories-add = [TH] Add Category [/TH]
categories-name = [TH] Name [/TH]
categories-name-placeholder =
    .placeholder = [TH] e.g. Beverages [/TH]
categories-colour = [TH] Colour [/TH]
categories-icon = [TH] Icon [/TH]
categories-id-preview = [TH] ID Preview [/TH]
categories-edit = [TH] Edit [/TH]
categories-create = [TH] Create [/TH]
categories-save = [TH] Save [/TH]
categories-delete-confirm = [TH] Delete Category [/TH]
categories-delete-warning = [TH] This will unlink all products in this category. [/TH]
categories-preview = [TH] Preview [/TH]
categories-name-aria =
    .aria-label = [TH] Category Name [/TH]
categories-icon-picker-aria =
    .aria-label = [TH] Pick an icon [/TH]
categories-colour-picker-aria =
    .aria-label = [TH] Pick a colour [/TH]
categories-icon-food = [TH] Food [/TH]
categories-icon-snack = [TH] Snack [/TH]
categories-icon-hot-drink = [TH] Hot drink [/TH]
categories-icon-cold-drink = [TH] Cold drink [/TH]
categories-icon-generic = [TH] Generic [/TH]

