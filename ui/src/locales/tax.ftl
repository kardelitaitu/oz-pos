# ui/src/locales/tax.ftl — Tax configuration

tax-config-title = Tax Configuration
tax-config-add = Add Tax Rate
tax-config-empty = No tax rates configured
tax-config-loading = Loading tax rates…
tax-config-col-name = Name
tax-config-col-rate = Rate (%)
tax-config-modal-title = { $editing ->
    [true] Edit Tax Rate
   *[other] Add Tax Rate
}
tax-config-field-name = Tax Name
tax-config-field-rate = Rate (%)
tax-config-btn-cancel = Cancel
tax-config-btn-save = Save
tax-config-btn-delete = Delete
tax-config-col-type = Type
tax-config-col-actions =
    .aria-label = Actions
tax-config-table-aria = Tax rates
tax-config-cat-table-aria = Category tax rates
tax-config-field-name-aria = Tax Name
tax-config-col-default = Default
tax-config-col-category = Category
tax-config-col-assigned = Assigned Tax Rates
tax-config-default-badge = Default
tax-config-type-inclusive = Inclusive
tax-config-type-exclusive = Exclusive
tax-config-yes = Yes
tax-config-edit = Edit
tax-config-edit-aria =
    .aria-label = Edit { $name }
tax-config-delete-aria =
    .aria-label = Delete { $name }
tax-config-cat-title = Category Tax Rates
tax-config-cat-desc = Assign default tax rates to product categories. Products inherit their category&rsquo;s tax rates unless overridden at the product level.
tax-config-no-categories = No categories available.
tax-config-no-rates-assigned = No rates assigned
tax-config-cat-edit-aria =
    .aria-label = Edit tax rates for { $name }
tax-config-modal-aria = { $editing ->
    [true] Edit tax rate
   *[other] Add tax rate
}
tax-config-field-name-placeholder =
    .placeholder = e.g. Sales Tax
tax-config-field-rate-placeholder =
    .placeholder = 825
tax-config-rate-hint = Enter rate in basis points (e.g. 825 = 8.25%)
tax-config-tax-type = Tax Type
tax-config-tax-type-aria = Tax type
tax-config-type-exclusive-label = Exclusive
tax-config-type-exclusive-desc = Added at checkout
tax-config-type-inclusive-label = Inclusive
tax-config-type-inclusive-desc = Included in price
tax-config-set-default = Set as default tax rate
tax-config-cat-modal-aria = Tax rates for { $name }
tax-config-cat-modal-title = Tax Rates &mdash; { $name }
tax-config-cat-modal-desc = Select the tax rates that apply to all products in this category.
tax-config-no-rates = No tax rates available. Create one first.
tax-config-modal-close =
    .aria-label = Close
