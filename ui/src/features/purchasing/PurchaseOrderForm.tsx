import { useState, useCallback, useEffect } from 'react';
import {
  createPurchaseOrder,
  listSuppliers,
  type SupplierDto,
} from '@/api/purchasing';
import { Button } from '@/components/Button';
import './PurchaseOrderForm.css';

interface LineItem {
  sku: string;
  product_name: string;
  qty: number;
  unit_cost_minor: number;
}

interface Props {
  editingId: string | null;
  onClose: () => void;
  onSaved: () => void;
}

export default function PurchaseOrderForm({ editingId, onClose, onSaved }: Props) {
  const [suppliers, setSuppliers] = useState<SupplierDto[]>([]);
  const [poNumber, setPoNumber] = useState('');
  const [supplierId, setSupplierId] = useState('');
  const [expectedDate, setExpectedDate] = useState('');
  const [notes, setNotes] = useState('');
  const [lines, setLines] = useState<LineItem[]>([
    { sku: '', product_name: '', qty: 1, unit_cost_minor: 0 },
  ]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listSuppliers().then(setSuppliers).catch(() => {});
  }, []);

  const addLine = useCallback(() => {
    setLines((prev) => [...prev, { sku: '', product_name: '', qty: 1, unit_cost_minor: 0 }]);
  }, []);

  const removeLine = useCallback((idx: number) => {
    setLines((prev) => prev.filter((_, i) => i !== idx));
  }, []);

  const updateLine = useCallback((idx: number, field: keyof LineItem, value: string | number) => {
    setLines((prev) => prev.map((line, i) => (i === idx ? { ...line, [field]: value } : line)));
  }, []);

  const subtotal = lines.reduce((sum, l) => sum + l.qty * l.unit_cost_minor, 0);

  const handleSave = useCallback(async () => {
    if (!poNumber.trim()) { setError('PO number is required'); return; }
    if (!supplierId) { setError('Supplier is required'); return; }
    if (lines.length === 0 || lines.some((l) => !l.sku.trim())) {
      setError('Each line must have a SKU');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const args: import('@/api/purchasing').CreatePurchaseOrderArgs = {
        po_number: poNumber.trim(),
        supplier_id: supplierId,
        lines: lines.map((l) => ({
          sku: l.sku.trim(),
          product_name: l.product_name.trim(),
          qty: l.qty,
          unit_cost_minor: l.unit_cost_minor,
        })),
      };
      if (expectedDate) args.expected_date = expectedDate;
      if (notes) args.notes = notes;
      await createPurchaseOrder(args);
      onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create purchase order');
    } finally {
      setSaving(false);
    }
  }, [poNumber, supplierId, expectedDate, notes, lines, onSaved]);

  return (
    <div className="po-form-overlay" role="dialog" aria-modal="true" aria-label="Purchase Order Form">
      <div className="po-form-modal">
        <div className="po-form-header">
          <h2>{editingId ? 'Edit Purchase Order' : 'New Purchase Order'}</h2>
          <button type="button" className="po-form-close" onClick={onClose} aria-label="Close">&times;</button>
        </div>

        <div className="po-form-body">
          <div className="po-form-row">
            <label className="po-form-field">
              <span className="po-form-label">PO Number *</span>
              <input className="po-form-input" type="text" value={poNumber} onChange={(e) => setPoNumber(e.target.value)} placeholder="PO-001" />
            </label>
            <label className="po-form-field">
              <span className="po-form-label">Supplier *</span>
              <select className="po-form-input po-form-select" value={supplierId} onChange={(e) => setSupplierId(e.target.value)}>
                <option value="">-- Select --</option>
                {suppliers.map((s) => (
                  <option key={s.id} value={s.id}>{s.name} ({s.code})</option>
                ))}
              </select>
            </label>
          </div>

          <div className="po-form-row">
            <label className="po-form-field">
              <span className="po-form-label">Expected Date</span>
              <input className="po-form-input" type="date" value={expectedDate} onChange={(e) => setExpectedDate(e.target.value)} />
            </label>
            <label className="po-form-field">
              <span className="po-form-label">Notes</span>
              <input className="po-form-input" type="text" value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="Optional notes" />
            </label>
          </div>

          <div className="po-form-section">
            <div className="po-form-section-header">
              <span className="po-form-label">Line Items</span>
              <Button variant="secondary" size="sm" onClick={addLine}>+ Add Line</Button>
            </div>

            <table className="po-form-lines-table" aria-label="Line items">
              <thead>
                <tr>
                  <th>SKU</th>
                  <th>Product Name</th>
                  <th>Qty</th>
                  <th>Unit Cost</th>
                  <th>Line Total</th>
                  <th aria-label="Actions"> </th>
                </tr>
              </thead>
              <tbody>
                {lines.map((line, idx) => (
                  <tr key={idx}>
                    <td>
                      <input
                        className="po-form-input po-form-input--sm"
                        type="text"
                        value={line.sku}
                        onChange={(e) => updateLine(idx, 'sku', e.target.value)}
                        placeholder="SKU"
                      />
                    </td>
                    <td>
                      <input
                        className="po-form-input po-form-input--sm"
                        type="text"
                        value={line.product_name}
                        onChange={(e) => updateLine(idx, 'product_name', e.target.value)}
                        placeholder="Product name"
                      />
                    </td>
                    <td>
                      <input
                        className="po-form-input po-form-input--sm po-form-input--num"
                        type="number"
                        min={0}
                        value={line.qty}
                        onChange={(e) => updateLine(idx, 'qty', parseInt(e.target.value) || 0)}
                      />
                    </td>
                    <td>
                      <input
                        className="po-form-input po-form-input--sm po-form-input--num"
                        type="number"
                        min={0}
                        value={line.unit_cost_minor}
                        onChange={(e) => updateLine(idx, 'unit_cost_minor', parseInt(e.target.value) || 0)}
                        placeholder="in cents"
                      />
                    </td>
                    <td className="po-form-line-total">
                      {(line.qty * line.unit_cost_minor / 100).toFixed(2)}
                    </td>
                    <td>
                      {lines.length > 1 && (
                        <button type="button" className="po-form-remove-line" onClick={() => removeLine(idx)} aria-label="Remove line">&times;</button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
              <tfoot>
                <tr>
                  <td colSpan={4} className="po-form-total-label">Subtotal</td>
                  <td className="po-form-total-value">{(subtotal / 100).toFixed(2)}</td>
                  <td />
                </tr>
              </tfoot>
            </table>
          </div>

          {error && <div className="po-form-error" role="alert">{error}</div>}
        </div>

        <div className="po-form-actions">
          <Button variant="ghost" onClick={onClose} disabled={saving}>Cancel</Button>
          <Button variant="primary" loading={saving} disabled={!poNumber.trim() || !supplierId} onClick={handleSave}>
            Create PO
          </Button>
        </div>
      </div>
    </div>
  );
}
