import { useLocalization } from '@fluent/react';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import TableManagementScreen from '@/features/tables/TableManagementScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import type { ProductDto } from '@/api/products';

// ── Sales History sub-view ──────────────────────────────────────────

interface SalesHistoryViewProps {
  theme: string | undefined;
  onBack: () => void;
}

/** Sales History full-screen sub-view with back button. */
export function SalesHistoryView({ theme, onBack }: SalesHistoryViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <header className="retail-header" style={{ justifyContent: 'space-between' }}>
        <div className="retail-header-store">
          <span className="retail-header-name">{l10n.getString('retail-fn-history') || 'Sales History'}</span>
        </div>
        <button
          className="retail-options-tab retail-options-tab--danger"
          onClick={onBack}
        >
          &larr; {l10n.getString('back')}
        </button>
      </header>
      <div style={{ flex: 1, overflow: 'auto' }}>
        <SalesHistoryScreen />
      </div>
    </div>
  );
}

// ── Table Management sub-view ──────────────────────────────────────

interface TableManagementViewProps {
  theme: string | undefined;
  onBack: () => void;
}

/** Table Management full-screen sub-view with back button. */
export function TableManagementView({ theme, onBack }: TableManagementViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <header className="retail-header" style={{ justifyContent: 'space-between' }}>
        <div className="retail-header-store">
          <span className="retail-header-name">{l10n.getString('tables-title') || 'Table Management'}</span>
        </div>
        <button
          className="retail-options-tab retail-options-tab--danger"
          onClick={onBack}
        >
          &larr; {l10n.getString('back')}
        </button>
      </header>
      <div style={{ flex: 1, overflow: 'auto' }}>
        <TableManagementScreen />
      </div>
    </div>
  );
}

// ── Stock Inquiry sub-view ─────────────────────────────────────────

interface StockInquiryViewProps {
  theme: string | undefined;
  onBack: () => void;
  onAddProduct: (p: ProductDto) => void;
}

/** Stock Inquiry full-screen sub-view with back button and product-add callback. */
export function StockInquiryView({ theme, onBack, onAddProduct }: StockInquiryViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <header className="retail-header" style={{ justifyContent: 'space-between' }}>
        <div className="retail-header-store">
          <span className="retail-header-name">{l10n.getString('retail-fn-stok') || 'Stock Inquiry'}</span>
        </div>
        <button
          className="retail-options-tab retail-options-tab--danger"
          onClick={onBack}
        >
          &larr; {l10n.getString('back')}
        </button>
      </header>
      <div style={{ flex: 1, overflow: 'auto' }}>
        <ProductLookupScreen onAddProduct={(p) => onAddProduct({
          sku: p.sku, name: p.name, category: p.category,
          price: p.price, barcode: p.barcode ?? null,
          in_stock: p.inStock, stock_qty: p.stockQty ?? null,
          product_type: p.productType,
          tax_rate_ids: [], created_at: '', price_updated_at: '',
        })} />
      </div>
    </div>
  );
}
