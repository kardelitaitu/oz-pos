# ui/src/locales/products.ftl — Product management, lookup, variants

# Product Lookup
product-lookup-title = Products
product-lookup-dev-fallback = Using sample data (IPC unavailable)
product-lookup-search-placeholder = Search products…
product-lookup-barcode-placeholder = Scan barcode…
product-lookup-barcode-scan = Scan
product-lookup-no-results = No products found
product-lookup-loading = Loading products…
product-lookup-add = Add to cart
product-lookup-in-stock = In stock
product-lookup-out-of-stock = Out of stock
product-lookup-all-categories = All Categories

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
variant-mgmt-error-load = Failed to load variants
variant-mgmt-error-save = Failed to save variant
variant-mgmt-error-delete = Failed to delete variant
