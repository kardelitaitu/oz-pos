import { useLocalization } from '@fluent/react';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import TableManagementScreen from '@/features/tables/TableManagementScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import RetailHeader from './RetailHeader';
import type { ProductDto } from '@/api/products';

// ── Sales History sub-view ──────────────────────────────────────────

interface SalesHistoryViewProps {
  theme: string | undefined;
  onBack: () => void;
}

/** Sales History full-screen sub-view — reuses RetailHeader in minimal variant. */
export function SalesHistoryView({ theme, onBack }: SalesHistoryViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <RetailHeader
        variant="minimal"
        title={l10n.getString('retail-fn-history') || 'Sales History'}
        onBack={onBack}
        skipTarget="retail-subview-main"
      />
      <div id="retail-subview-main" style={{ flex: 1, overflow: 'auto' }}>
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

/** Table Management full-screen sub-view — reuses RetailHeader in minimal variant. */
export function TableManagementView({ theme, onBack }: TableManagementViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <RetailHeader
        variant="minimal"
        title={l10n.getString('tables-title') || 'Table Management'}
        onBack={onBack}
        skipTarget="retail-subview-main"
      />
      <div id="retail-subview-main" style={{ flex: 1, overflow: 'auto' }}>
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

/** Stock Inquiry full-screen sub-view — reuses RetailHeader in minimal variant. */
export function StockInquiryView({ theme, onBack, onAddProduct }: StockInquiryViewProps) {
  const { l10n } = useLocalization();
  return (
    <div className="retail-pos" data-theme={theme}>
      <RetailHeader
        variant="minimal"
        title={l10n.getString('retail-fn-stok') || 'Stock Inquiry'}
        onBack={onBack}
        skipTarget="retail-subview-main"
      />
      <div id="retail-subview-main" style={{ flex: 1, overflow: 'auto' }}>
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
