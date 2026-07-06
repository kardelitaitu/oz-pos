import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import {
  listProducts,
  createProduct,
  updateProduct,
  deleteProduct,
  listCategories,
  type ProductDto,
  type CategoryDto,
} from '@/api/products';
import { listTaxRates, type TaxRateDto } from '@/api/tax';
import { listCurrencies, type CurrencyDto } from '@/api/currency';
import { formatMoney, type Product, type Sku } from '@/types/domain';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import VariantManagementScreen from './VariantManagementScreen';
import './ProductManagementScreen.css';

interface FormData {
  sku: string;
  name: string;
  priceMinor: string;
  currency: string;
  categoryId: string;
  barcode: string;
  initialStock: string;
  taxRateIds: string[];
}

const EMPTY_FORM: FormData = {
  sku: '',
  name: '',
  priceMinor: '',
  currency: 'USD',
  categoryId: '',
  barcode: '',
  initialStock: '0',
  taxRateIds: [],
};

function dtoToProduct(dto: ProductDto): Product {
  return {
    sku: dto.sku as Sku,
    name: dto.name,
    category: dto.category ?? 'Uncategorised',
    price: { minor_units: dto.price.minor_units, currency: dto.price.currency },
    barcode: dto.barcode,
    inStock: dto.in_stock,
    stockQty: dto.stock_qty,
    priceUpdatedAt: dto.price_updated_at,
  };
}

export default function ProductManagementScreen() {
  const { session } = useAuth();
  const userId = session?.user_id ?? '';
  const [products, setProducts] = useState<Product[]>([]);
  const [productDtos, setProductDtos] = useState<ProductDto[]>([]);
  const [taxRates, setTaxRates] = useState<TaxRateDto[]>([]);
  const [categories, setCategories] = useState<CategoryDto[]>([]);
  const [currencies, setCurrencies] = useState<CurrencyDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editingSku, setEditingSku] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [variantProductSku, setVariantProductSku] = useState<string | null>(null);
  const [variantProductName, setVariantProductName] = useState<string>('');

  const { l10n } = useLocalization();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [dtos, rates, cats, currencyList] = await Promise.all([listProducts(), listTaxRates(), listCategories(), listCurrencies()]);
      setProductDtos(dtos);
      setProducts(dtos.map(dtoToProduct));
      setTaxRates(rates);
      setCategories(cats);
      setCurrencies(currencyList);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const openCreate = useCallback(() => {
    setForm(EMPTY_FORM);
    setEditingSku(null);
    setShowModal(true);
  }, []);

  const openEdit = useCallback((p: Product) => {
    const dto = productDtos.find((d) => d.sku === p.sku);
    setForm({
      sku: p.sku,
      name: p.name,
      priceMinor: String(p.price.minor_units),
      currency: p.price.currency,
      categoryId: p.category === 'Uncategorised' ? '' : p.category,
      barcode: p.barcode ?? '',
      initialStock: '0',
      taxRateIds: dto?.tax_rate_ids ?? [],
    });
    setEditingSku(p.sku);
    setShowModal(true);
  }, [productDtos]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const priceMinor = parseInt(form.priceMinor, 10);
      if (Number.isNaN(priceMinor) || priceMinor < 0) return;

      if (editingSku) {
        await updateProduct({
          userId,
          sku: editingSku,
          name: form.name,
          priceMinor,
          currency: form.currency,
          categoryId: form.categoryId || undefined,
          barcode: form.barcode || undefined,
          taxRateIds: form.taxRateIds,
        });
      } else {
        await createProduct({
          userId,
          sku: form.sku,
          name: form.name,
          priceMinor,
          currency: form.currency,
          categoryId: form.categoryId || undefined,
          barcode: form.barcode || undefined,
          initialStock: parseInt(form.initialStock, 10) || 0,
          taxRateIds: form.taxRateIds,
        });
      }
      setShowModal(false);
      await load();
    } catch {
      // Error handling.
    } finally {
      setSaving(false);
    }
  }, [form, editingSku, load]);

  const confirmDelete = useCallback(async (sku: string) => {
    setDeleting(sku);
    try {
      await deleteProduct({ userId, sku });
      setDeleting(null);
      await load();
    } catch {
      setDeleting(null);
    }
  }, [load, userId]);

  return (
    <div className="product-mgmt">
      <div className="product-mgmt-header">
        <Localized id="product-mgmt-title">
          <h1 className="product-mgmt-title">Products</h1>
        </Localized>
        <Localized id="product-mgmt-add">
          <Button onClick={openCreate}>Add Product</Button>
        </Localized>
      </div>

      {loading ? (
        <Localized id="product-mgmt-loading">
          <p className="product-mgmt-loading">Loading products…</p>
        </Localized>
      ) : products.length === 0 ? (
        <Card shadow="sm">
          <div className="product-mgmt-empty">
            <Localized id="product-mgmt-empty">
              <p>No products yet.</p>
            </Localized>
            <Localized id="product-mgmt-empty-cta">
              <Button variant="secondary" onClick={openCreate}>Add your first product</Button>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="product-mgmt-table-wrap">
          <Localized id="product-mgmt-table-aria" attrs={{ 'aria-label': true }}>
          <table className="product-mgmt-table" aria-label="Product catalog">
            <thead>
              <tr>
                <Localized id="product-mgmt-col-sku"><th>SKU</th></Localized>
                <Localized id="product-mgmt-col-name"><th>Name</th></Localized>
                <Localized id="product-mgmt-col-category"><th>Category</th></Localized>
                <Localized id="product-mgmt-col-price"><th>Price</th></Localized>
                <Localized id="product-mgmt-col-barcode"><th>Barcode</th></Localized>
                <Localized id="product-mgmt-col-stock"><th>Stock</th></Localized>
                <Localized id="product-mgmt-actions-aria" attrs={{ 'aria-label': true }}>
                  <th aria-label="Actions"> </th>
                </Localized>
              </tr>
            </thead>
            <tbody>
              {products.map((p) => (
                <tr key={p.sku}>
                  <td className="product-mgmt-cell-sku">{p.sku}</td>
                  <td>{p.name}</td>
                  <td>{p.category}</td>
                  <td className="product-mgmt-cell-price">{formatMoney(p.price)}</td>
                  <td className="product-mgmt-cell-barcode">{p.barcode ?? '\u2014'}</td>
                  <td>
                    {p.stockQty != null && p.stockQty < 10 ? (
                      <span className="product-mgmt-stock-low" style={{ color: 'var(--color-danger)', fontWeight: 600 }}>
                        {p.stockQty}
                      </span>
                    ) : p.stockQty != null ? (
                      <span>{p.stockQty}</span>
                    ) : p.inStock ? (
                      <Localized id="product-mgmt-stock-in">
                        <span className="product-mgmt-stock-ok">In stock</span>
                      </Localized>
                    ) : (
                      <Localized id="product-mgmt-stock-out">
                        <span className="product-mgmt-stock-low">Out of stock</span>
                      </Localized>
                    )}
                  </td>
                  <td className="product-mgmt-cell-actions">
                    <Localized id="product-mgmt-variants-aria" attrs={{ 'aria-label': true }} vars={{ name: p.name }}>
                      <button
                        type="button"
                        className="product-mgmt-action-btn"
                        onClick={() => {
                          setVariantProductSku(p.sku);
                          setVariantProductName(p.name);
                        }}
                        aria-label={`Variants for ${p.name}`}
                      >
                        <Localized id="product-mgmt-variants">
                          <span>Variants</span>
                        </Localized>
                      </button>
                    </Localized>
                    <Localized id="product-mgmt-edit-aria" attrs={{ 'aria-label': true }} vars={{ name: p.name }}>
                      <button
                        type="button"
                        className="product-mgmt-action-btn"
                        onClick={() => openEdit(p)}
                        aria-label={`Edit ${p.name}`}
                      >
                        <Localized id="product-mgmt-edit">
                          <span>Edit</span>
                        </Localized>
                      </button>
                    </Localized>
                    <Localized id="product-mgmt-delete-aria" attrs={{ 'aria-label': true }} vars={{ name: p.name }}>
                      <button
                        type="button"
                        className="product-mgmt-action-btn product-mgmt-action-btn--danger"
                        onClick={() => confirmDelete(p.sku)}
                        disabled={deleting === p.sku}
                        aria-label={`Delete ${p.name}`}
                      >
                        <Localized id="product-mgmt-delete">
                          <span>Delete</span>
                        </Localized>
                      </button>
                    </Localized>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          </Localized>
        </div>
      )}

      {showModal && (
        <Localized id="product-mgmt-modal-aria" attrs={{ 'aria-label': true }} vars={{ mode: editingSku ? 'edit' : 'add' }}>
        <div className="product-mgmt-overlay" role="dialog" aria-modal="true" aria-label={editingSku ? 'Edit product' : 'Add product'}>
          <div className="product-mgmt-modal">
            <div className="product-mgmt-modal-header">
              <Localized id={editingSku ? 'product-mgmt-modal-edit-title' : 'product-mgmt-modal-add-title'}>
                <h2>{editingSku ? 'Edit Product' : 'Add Product'}</h2>
              </Localized>
              <Localized id="product-mgmt-modal-close-aria" attrs={{ 'aria-label': true }}>
                <button
                  type="button"
                  className="product-mgmt-modal-close"
                  onClick={() => setShowModal(false)}
                  aria-label="Close"
                >
                  &times;
                </button>
              </Localized>
            </div>

            <div className="product-mgmt-modal-body">
              <label className="product-mgmt-field" htmlFor="product-field-sku">
                {l10n.getString('product-mgmt-field-sku-required')}
                <Localized id="product-mgmt-sku-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="product-mgmt-input"
                    type="text"
                    id="product-field-sku"
                    value={form.sku}
                    onChange={(e) => setForm({ ...form, sku: e.target.value })}
                    disabled={!!editingSku}
                    placeholder="e.g. LATTE"
                  />
                </Localized>
              </label>

              <label className="product-mgmt-field" htmlFor="product-field-name">
                {l10n.getString('product-mgmt-field-name-required')}
                <Localized id="product-mgmt-name-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="product-mgmt-input"
                    type="text"
                    id="product-field-name"
                    value={form.name}
                    onChange={(e) => setForm({ ...form, name: e.target.value })}
                    placeholder="e.g. Caffè Latte"
                  />
                </Localized>
              </label>

              <div className="product-mgmt-row">
                <label className="product-mgmt-field" htmlFor="product-field-price">
                  {l10n.getString('product-mgmt-field-price')}
                  <Localized id="product-mgmt-price-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="product-mgmt-input"
                      type="number"
                      id="product-field-price"
                      min="0"
                      value={form.priceMinor}
                      onChange={(e) => setForm({ ...form, priceMinor: e.target.value })}
                      placeholder="450"
                    />
                  </Localized>
                </label>

                <label className="product-mgmt-field" htmlFor="product-field-currency">
                  <Localized id="product-mgmt-field-currency">
                    <span className="product-mgmt-label">Currency</span>
                  </Localized>
                  <select
                    className="product-mgmt-input product-mgmt-select"
                    id="product-field-currency"
                    value={form.currency}
                    onChange={(e) => setForm({ ...form, currency: e.target.value })}
                  >
                    {currencies.map((c) => (
                      <option key={c.code} value={c.code}>{c.code} — {c.symbol}</option>
                    ))}
                  </select>
                </label>
              </div>

              <label className="product-mgmt-field" htmlFor="product-field-category">
                <Localized id="product-mgmt-field-category">
                  <span className="product-mgmt-label">Category</span>
                </Localized>
                <select
                  className="product-mgmt-input product-mgmt-select"
                  id="product-field-category"
                  value={form.categoryId}
                  onChange={(e) => setForm({ ...form, categoryId: e.target.value })}
                >
                  <Localized id="product-mgmt-no-category">
                    <option value="">— No category —</option>
                  </Localized>
                  {categories.map((cat) => (
                    <option key={cat.id} value={cat.id}>{cat.name}</option>
                  ))}
                </select>
              </label>

              <label className="product-mgmt-field" htmlFor="product-field-barcode">
                {l10n.getString('product-mgmt-field-barcode')}
                <Localized id="product-mgmt-barcode-placeholder" attrs={{ placeholder: true }}>
                  <input
                    className="product-mgmt-input"
                    type="text"
                    id="product-field-barcode"
                    value={form.barcode}
                    onChange={(e) => setForm({ ...form, barcode: e.target.value })}
                    placeholder="4901234567890"
                  />
                </Localized>
              </label>

              {taxRates.length > 0 && (
                <fieldset className="product-mgmt-field">
                  <Localized id="product-mgmt-field-tax-rates">
                    <legend className="product-mgmt-label">Tax Rates</legend>
                  </Localized>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-1)', marginTop: 'var(--space-1)' }}>
                    {taxRates.map((tr) => (
                      <label
                        key={tr.id}
                        style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', cursor: 'pointer', fontSize: 'var(--text-sm)' }}
                      >
                        <input
                          type="checkbox"
                          checked={form.taxRateIds.includes(tr.id)}
                          onChange={(e) => {
                            setForm({
                              ...form,
                              taxRateIds: e.target.checked
                                ? [...form.taxRateIds, tr.id]
                                : form.taxRateIds.filter((id) => id !== tr.id),
                            });
                          }}
                        />
                        {tr.name} ({tr.display_rate})
                      </label>
                    ))}
                  </div>
                </fieldset>
              )}

              {!editingSku && (
                <label className="product-mgmt-field" htmlFor="product-field-stock">
                  {l10n.getString('product-mgmt-field-stock')}
                  <Localized id="product-mgmt-stock-placeholder" attrs={{ placeholder: true }}>
                    <input
                      className="product-mgmt-input"
                      type="number"
                      id="product-field-stock"
                      min="0"
                      value={form.initialStock}
                      onChange={(e) => setForm({ ...form, initialStock: e.target.value })}
                      placeholder="0"
                    />
                  </Localized>
                </label>
              )}
            </div>

            <div className="product-mgmt-modal-actions">
              <Localized id="product-mgmt-btn-cancel">
                <Button variant="ghost" onClick={() => setShowModal(false)} disabled={saving}>Cancel</Button>
              </Localized>
              <Button
                variant="primary"
                loading={saving}
                disabled={!form.sku.trim() || !form.name.trim()}
                onClick={handleSave}
              >
                <Localized id={editingSku ? 'product-mgmt-btn-update' : 'product-mgmt-btn-create'}>
                  <span>{editingSku ? 'Update' : 'Create'}</span>
                </Localized>
              </Button>
            </div>
          </div>
        </div>
        </Localized>
      )}

      {variantProductSku && (
        <VariantManagementScreen
          productSku={variantProductSku}
          productName={variantProductName}
          onClose={() => setVariantProductSku(null)}
        />
      )}
    </div>
  );
}
