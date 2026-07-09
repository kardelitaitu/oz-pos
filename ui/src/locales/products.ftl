# ui/src/locales/products.ftl — Product management, lookup, variants

# ── Restaurant Menu ──────────────────────────────────────────────────
restaurant-menu-search-placeholder =
    .placeholder = Search Menu
restaurant-menu-hamburger-aria =
    .aria-label = Menu
restaurant-menu-back-aria =
    .aria-label = Back to workspaces
restaurant-size-decrease-aria =
    .aria-label = Decrease size
restaurant-size-increase-aria =
    .aria-label = Increase size
restaurant-font-size-decrease-aria =
    .aria-label = Decrease font size
restaurant-font-size-increase-aria =
    .aria-label = Increase font size
restaurant-font-size-label = Font Size
restaurant-theme-light = Light Mode
restaurant-theme-dark = Dark Mode
restaurant-lock-terminal = Lock Terminal
restaurant-toggle-fullscreen = Toggle Fullscreen
restaurant-clear-color-aria =
    .aria-label = Clear color
restaurant-categories-aria =
    .aria-label = Menu categories
restaurant-menu-loading = Loading menu…
restaurant-menu-empty = Menu is empty
restaurant-size-label = Size
restaurant-sort-label = Sort
restaurant-card-add = Add
restaurant-context-color-label = Color

# Product Lookup
product-lookup-title = Products
product-lookup-dev-fallback = Using sample data (IPC unavailable)
product-lookup-search-placeholder =
    .placeholder = Search products…
product-lookup-search-aria =
    .aria-label = Search for products by name, SKU, or barcode
product-lookup-barcode-placeholder =
    .placeholder = Scan barcode…
product-lookup-barcode-aria =
    .aria-label = Enter or scan a barcode
product-lookup-barcode-scan = Scan
product-lookup-scan-btn-aria =
    .aria-label = Submit the entered barcode
product-lookup-no-results = No products found
product-lookup-loading = Loading products…
product-lookup-add = Add to cart
product-lookup-in-stock = In stock
product-lookup-out-of-stock = Out of stock
product-lookup-all-categories = All Categories
product-lookup-categories-aria = Filter by category
product-lookup-grid-aria = Product search results
product-lookup-card-aria =
    .aria-label = { $name } — { $price }. SKU: { $sku }. { $stock }
product-lookup-bundle-added = Bundle "{ $name }" added — { $count } items
product-lookup-no-match = No product or bundle matches this barcode
product-lookup-uncategorised = Uncategorised
product-lookup-error-load = Failed to load products

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
product-mgmt-table-aria = Product catalog
product-mgmt-actions-aria =
    .aria-label = Actions
product-mgmt-variants = Variants
product-mgmt-variants-aria =
    .aria-label = Variants for { $name }
product-mgmt-modal-aria =
    .aria-label = { $mode ->
        [add] Add product
       *[edit] Edit product
    }
product-mgmt-modal-close-aria =
    .aria-label = Close
product-mgmt-sku-placeholder =
    .placeholder = e.g. LATTE
product-mgmt-name-placeholder =
    .placeholder = e.g. Caffè Latte
product-mgmt-price-placeholder =
    .placeholder = 450
product-mgmt-barcode-placeholder =
    .placeholder = 4901234567890
product-mgmt-stock-placeholder =
    .placeholder = 0
product-mgmt-no-category = — No category —
product-mgmt-col-type = Type
product-mgmt-field-type = Type

# Product Variants
variant-mgmt-title = Variants — { $product }
variant-mgmt-loading = Loading variants…
variant-mgmt-empty = No variants yet.
variant-mgmt-empty-cta = Add a variant
variant-mgmt-add = Add Variant
variant-mgmt-col-name = Name
variant-mgmt-col-sku = SKU
variant-mgmt-col-price = Price
variant-mgmt-col-barcode = Barcode
variant-mgmt-col-status = Status
variant-mgmt-price-parent = Uses parent price
variant-mgmt-status-active = Active
variant-mgmt-status-inactive = Inactive
variant-mgmt-edit = Edit
variant-mgmt-edit-aria = Edit { $name }
variant-mgmt-delete = Delete
variant-mgmt-delete-aria = Delete { $name }
variant-mgmt-delete-confirm-title = Delete Variant
variant-mgmt-delete-confirm-body = Are you sure you want to delete variant "{ $name }" ({ $sku })? This action cannot be undone.
variant-mgmt-delete-confirm-cancel = Cancel
variant-mgmt-delete-confirm-confirm = Delete
variant-mgmt-modal-add-title = Add Variant
variant-mgmt-modal-edit-title = Edit Variant
variant-mgmt-modal-close = Close
variant-mgmt-close = Close
variant-mgmt-close-aria =
    .aria-label = Close
variant-mgmt-field-name-required = Name *
variant-mgmt-field-sku-required = SKU *
variant-mgmt-field-price = Price (minor units)
variant-mgmt-field-currency = Currency
variant-mgmt-field-barcode = Barcode
variant-mgmt-field-sort-order = Sort order
variant-mgmt-field-active = Active
variant-mgmt-btn-cancel = Cancel
variant-mgmt-btn-create = Create
variant-mgmt-btn-update = Update
variant-mgmt-overlay-aria = Variants for { $name }
variant-mgmt-dialog-aria = { $mode ->
    [add] Add variant
   *[edit] Edit variant
}
variant-mgmt-table-aria = Product variants
variant-mgmt-actions-aria =
    .aria-label = Actions
variant-mgmt-name-placeholder =
    .placeholder = e.g. Large
variant-mgmt-sku-placeholder =
    .placeholder = e.g. TEA-LARGE
variant-mgmt-price-placeholder =
    .placeholder = 450
variant-mgmt-currency-placeholder =
    .placeholder = USD
variant-mgmt-barcode-placeholder =
    .placeholder = 4901234567890
variant-mgmt-sort-placeholder =
    .placeholder = 0
variant-mgmt-delete-confirm-aria =
    .aria-label = Delete confirmation
variant-mgmt-error-load = Failed to load variants
variant-mgmt-error-save = Failed to save variant
variant-mgmt-error-delete = Failed to delete variant

# Category Management
categories-title = Categories
categories-loading = Loading categories…
categories-no-categories = No categories yet
categories-empty-desc = Get started by creating your first category
categories-add-first = Add your first category
categories-add = Add Category
categories-name = Name
categories-name-placeholder =
    .placeholder = e.g. Beverages
categories-colour = Colour
categories-icon = Icon
categories-id-preview = ID Preview
categories-edit = Edit
categories-create = Create
categories-save = Save
categories-delete-confirm = Delete Category
categories-delete-warning = This will unlink all products in this category.
categories-preview = Preview
