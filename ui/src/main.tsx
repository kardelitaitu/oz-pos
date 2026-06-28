import React from 'react';
import ReactDOM from 'react-dom/client';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import App from './App';
import './styles/reset.css';
import './styles/tokens.css';
import './styles/components.css';

// Bootstrap the only locale we ship at scaffold time. Adding a new
// locale is a copy of en-US.ftl + a translation; the registration
// list below grows accordingly.
const bundle = new FluentBundle('en-US');
bundle.addResource(
  new FluentResource(
    // Inline import keeps the scaffold runnable without a bundler
    // plugin for .ftl files. The real loader is added in a follow-up.
    [
      'cart-title = Cart',
      'cart-empty = Cart is empty',
      'cart-line-remove = Remove',
      'cart-total-label = Total',
      'sale-pay-button = Pay',
      'sale-pay-button-aria = Charge the customer for the current cart',
      'product-lookup-title = Products',
      'product-lookup-search-placeholder = Search products…',
      'product-lookup-barcode-placeholder = Scan barcode…',
      'product-lookup-barcode-scan = Scan',
      'product-lookup-no-results = No products found',
      'product-lookup-loading = Loading products…',
      'product-lookup-add = Add to cart',
      'product-lookup-in-stock = In stock',
      'product-lookup-out-of-stock = Out of stock',
      'product-lookup-all-categories = All Categories',
      'badge-default = Default',
      'badge-success = Success',
      'badge-warning = Warning',
      'badge-danger = Danger',
      'badge-info = Info',
      'spinner-label = Loading…',
      'toast-success = Operation completed successfully',
      'toast-error = Something went wrong',
      'toast-warning = Please check your input',
      'toast-info = This is an informational message',
      'empty-state-title = Nothing here yet',
      'empty-state-desc = Get started by adding your first item',
      'empty-state-cta = Add Product',
      'error-state-title = Something went wrong',
      'error-state-desc = An unexpected error occurred. Please try again.',
      'error-state-retry = Retry',
      'pos-cart-panel-title = Current Sale',
      'pos-cart-empty = Cart is empty',
      'pos-cart-total = Total',
      'pos-cart-pay = Charge { $amount }',
    ].join('\n'),
  ),
);
const l10n = new ReactLocalization([bundle]);

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <LocalizationProvider l10n={l10n}>
      <App />
    </LocalizationProvider>
  </React.StrictMode>,
);
