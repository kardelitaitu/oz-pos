import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import {
  getReceiptSettings,
  setReceiptSettings,
  getStoreSettings,
  setStoreSettings,
  getHardwareSettings,
  setHardwareSettings,
  getCreditSettings,
  setCreditSettings,
  type ReceiptSettingsDto,
  type StoreSettingsDto,
  type HardwareSettingsDto,
  type CreditSettingsDto,
} from '@/api/settings';
import { listScanners, type ScannerInfo } from '@/api/hardware';
import './RetailPosScreen.css';

type TabId = 'general' | 'receipt' | 'printer' | 'scanner' | 'credit' | 'system';

interface RetailOptionsScreenProps {
  onClose: () => void;
}

const TABS: { id: TabId; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'receipt', label: 'Receipt' },
  { id: 'printer', label: 'Printer' },
  { id: 'scanner', label: 'Scanner' },
  { id: 'credit', label: 'Credit' },
  { id: 'system', label: 'System' },
];

// ── Paper dimensions (mm) ──────────────────────────────────────

const PAPER_DIMS: Record<string, { w: number; h: number }> = {
  narrow: { w: 58, h: 150 },
  standard: { w: 80, h: 150 },
  '9.5x11': { w: 241.3, h: 279.4 },
  '9.5x5.5': { w: 241.3, h: 139.7 },
  a4: { w: 210, h: 297 },
  letter: { w: 215.9, h: 279.4 },
};

const SCALE = 2.5; // px per mm for the preview popup

// ── Receipt Preview ────────────────────────────────────────────

function fmtPrice(
  minor: number,
  currency: string,
  showCurrency: boolean,
  decimalSep: string,
): string {
  const whole = Math.floor(Math.abs(minor) / 100);
  const frac = String(Math.abs(minor) % 100).padStart(2, '0');
  const sign = minor < 0 ? '-' : '';
  const sep = decimalSep === 'comma' ? ',' : decimalSep === 'none' ? '' : '.';
  const num = `${sign}${whole}${sep}${frac}`;
  return showCurrency ? `${currency} ${num}` : num;
}

function ReceiptContent({
  store, receipt, session,
}: {
  store: StoreSettingsDto;
  receipt: ReceiptSettingsDto;
  session: { display_name: string } | null;
}) {
  const currency = store.currency || 'IDR';
  const date = new Date().toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' });
  const time = new Date().toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' });

  const items = [
    { name: 'Indomie Goreng', qty: 2, price: 3500 },
    { name: 'Teh Botol Sosro', qty: 1, price: 5000 },
    { name: 'Nasi Goreng Spesial', qty: 1, price: 15000 },
  ];
  const subtotal = items.reduce((s, i) => s + i.price * i.qty, 0);
  const tax = Math.round(subtotal * 0.11);
  const total = subtotal + tax;

  return (
    <>
      <div className="retail-receipt-header">
        <strong>{store.name || 'Toko Anda'}</strong>
        <small>{store.address || 'Jl. Contoh No. 123'}</small>
        <small>{date} {time}</small>
        <small>Cashier: {session?.display_name ?? '-'}</small>
      </div>

      <div className="retail-receipt-divider" />

      <div className="retail-receipt-items">
        <div className="retail-receipt-items-head">
          <span>Item</span>
          <span>Qty</span>
          <span>Price</span>
        </div>
        {items.map((item, i) => (
          <div key={i} className="retail-receipt-item">
            <span className="retail-receipt-item-name">{item.name}</span>
            <span className="retail-receipt-item-qty">{item.qty}</span>
            <span className="retail-receipt-item-price">
              {fmtPrice(item.price * item.qty, currency, receipt.showCurrency, receipt.decimalSeparator)}
            </span>
          </div>
        ))}
      </div>

      <div className="retail-receipt-divider" />

      <div className="retail-receipt-totals">
        <div className="retail-receipt-total-line">
          <span>Subtotal</span>
          <span>{fmtPrice(subtotal, currency, receipt.showCurrency, receipt.decimalSeparator)}</span>
        </div>
        {receipt.showTax && (
          <div className="retail-receipt-total-line">
            <span>PPN (11%)</span>
            <span>{fmtPrice(tax, currency, receipt.showCurrency, receipt.decimalSeparator)}</span>
          </div>
        )}
        <div className="retail-receipt-total-line retail-receipt-total-line--grand">
          <span>TOTAL</span>
          <span>{fmtPrice(total, currency, receipt.showCurrency, receipt.decimalSeparator)}</span>
        </div>
      </div>

      {receipt.footer && (
        <>
          <div className="retail-receipt-divider" />
          <div className="retail-receipt-footer">{receipt.footer}</div>
        </>
      )}
    </>
  );
}

function getPaperDims(paperWidth: string) {
  return PAPER_DIMS[paperWidth] ?? PAPER_DIMS['standard']!;
}

function ReceiptPreview({
  store, receipt, session, scale = 1.2,
}: {
  store: StoreSettingsDto;
  receipt: ReceiptSettingsDto;
  session: { display_name: string } | null;
  scale?: number;
}) {
  const dims = getPaperDims(receipt.paperWidth);
  return (
    <div className="retail-receipt-preview" style={{
      width: dims.w * scale,
      padding: `${receipt.marginTop * scale}px ${receipt.marginRight * scale}px ${receipt.marginBottom * scale}px ${receipt.marginLeft * scale}px`,
    }}>
      <ReceiptContent store={store} receipt={receipt} session={session} />
    </div>
  );
}

export default function RetailOptionsScreen({ onClose }: RetailOptionsScreenProps) {
  const { addToast } = useToast();
  const { session } = useAuth();
  const roleId = session!.role_id;

  const [activeTab, setActiveTab] = useState<TabId>('general');
  const [saving, setSaving] = useState(false);
  const [showPreview, setShowPreview] = useState(false);

  // ── Store settings ────────────────────────────────────────────

  const [store, setStore] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  const [storeLoaded, setStoreLoaded] = useState(false);

  useEffect(() => {
    getStoreSettings()
      .then(setStore)
      .catch(() => {})
      .finally(() => setStoreLoaded(true));
  }, []);

  // ── Receipt settings ──────────────────────────────────────────

  const [receipt, setReceipt] = useState<ReceiptSettingsDto>({
    showCurrency: true, decimalSeparator: 'dot', showTax: true,
    footer: '', paperWidth: 'standard', showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  });
  const [receiptLoaded, setReceiptLoaded] = useState(false);

  useEffect(() => {
    getReceiptSettings()
      .then(setReceipt)
      .catch(() => {})
      .finally(() => setReceiptLoaded(true));
  }, []);

  // ── Credit settings ───────────────────────────────────────────

  const [credit, setCredit] = useState<CreditSettingsDto>({
    enabled: false, reminderIntervalHours: 24, maxLimitMinor: 0,
  });
  const [creditLoaded, setCreditLoaded] = useState(false);

  useEffect(() => {
    getCreditSettings()
      .then(setCredit)
      .catch(() => {})
      .finally(() => setCreditLoaded(true));
  }, []);

  // ── Hardware settings ─────────────────────────────────────────

  const [hardware, setHardware] = useState<HardwareSettingsDto>({
    printerConnection: 'auto',
    printerDevicePath: '',
    printerPaperSize: '80',
    scannerDeviceId: '',
    scannerInputMode: 'auto',
  });
  const [hardwareLoaded, setHardwareLoaded] = useState(false);

  useEffect(() => {
    getHardwareSettings()
      .then(setHardware)
      .catch(() => {})
      .finally(() => setHardwareLoaded(true));
  }, []);

  // ── Scanner list ──────────────────────────────────────────────

  const [scanners, setScanners] = useState<ScannerInfo[]>([]);
  useEffect(() => {
    listScanners().then(setScanners).catch(() => {});
  }, []);

  // ── Keyboard shortcuts ───────────────────────────────────────

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape' && !showPreview) {
        onClose();
      }
      if (e.key === 'Escape' && showPreview) {
        setShowPreview(false);
      }
    }
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onClose, showPreview]);

  // ── Save ──────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await setStoreSettings(store, roleId);
      await setReceiptSettings(receipt, roleId);
      await setCreditSettings(credit, roleId);
      await setHardwareSettings(hardware, roleId);
      addToast({ message: 'Settings saved', type: 'success' });
    } catch {
      addToast({ message: 'Failed to save settings', type: 'error' });
    } finally {
      setSaving(false);
    }
  }, [store, receipt, credit, hardware, roleId, addToast]);

  if (!storeLoaded || !receiptLoaded || !creditLoaded || !hardwareLoaded) {
    return (
      <div className="retail-pos">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1, color: '#888', fontSize: 14 }}>
          Loading…
        </div>
      </div>
    );
  }

  return (
    <div className="retail-pos">
      {/* ── Header ──────────────────────────── */}
      <header className="retail-header">
        <div className="retail-header-store">
          <span className="retail-header-name">SETTINGS</span>
        </div>
        <div className="retail-header-right">
          <span className="retail-header-clock">Options</span>
        </div>
      </header>

      {/* ── Body ────────────────────────────── */}
      <div className="retail-options-body">
        {/* Sidebar tabs */}
        <div className="retail-options-sidebar">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              className={`retail-options-tab${activeTab === tab.id ? ' retail-options-tab--active' : ''}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          ))}
          <div style={{ flex: 1 }} />
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={onClose}
          >
            &larr; Back
          </button>
        </div>

        {/* Content */}
        <div className="retail-options-content">
          {activeTab === 'general' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">General Settings</h3>
              <div className="retail-options-field">
                <label>Store Name</label>
                <input value={store.name} onChange={(e) => setStore({ ...store, name: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>Address</label>
                <input value={store.address} onChange={(e) => setStore({ ...store, address: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>Branch</label>
                <input value={store.branch} onChange={(e) => setStore({ ...store, branch: e.target.value })} placeholder="Main branch" />
              </div>
              <div className="retail-options-field">
                <label>Tax ID</label>
                <input value={store.taxId} onChange={(e) => setStore({ ...store, taxId: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>Currency</label>
                <select value={store.currency} onChange={(e) => setStore({ ...store, currency: e.target.value })}>
                  <option value="IDR">IDR (Rp)</option>
                  <option value="USD">USD ($)</option>
                  <option value="MYR">MYR (RM)</option>
                  <option value="SGD">SGD (S$)</option>
                  <option value="PHP">PHP (₱)</option>
                  <option value="THB">THB (฿)</option>
                  <option value="VND">VND (₫)</option>
                </select>
              </div>
            </div>
          )}

          {activeTab === 'receipt' && (
            <div className="retail-options-receipt-layout">
              <div className="retail-options-section">
                <h3 className="retail-options-heading">Receipt Settings</h3>
                <div className="retail-options-field retail-options-field--row">
                  <label>Show currency symbol</label>
                  <input
                    type="checkbox"
                    checked={receipt.showCurrency}
                    onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label>Show tax line</label>
                  <input
                    type="checkbox"
                    checked={receipt.showTax}
                    onChange={(e) => setReceipt({ ...receipt, showTax: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label>Show table number</label>
                  <input
                    type="checkbox"
                    checked={receipt.showTableNumber}
                    onChange={(e) => setReceipt({ ...receipt, showTableNumber: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>Decimal separator</label>
                  <select
                    value={receipt.decimalSeparator}
                    onChange={(e) => setReceipt({ ...receipt, decimalSeparator: e.target.value })}
                  >
                    <option value="dot">Dot (.)</option>
                    <option value="comma">Comma (,)</option>
                    <option value="none">None</option>
                  </select>
                </div>
                <div className="retail-options-field">
                  <label>Paper width</label>
                <select
                  value={receipt.paperWidth}
                  onChange={(e) => setReceipt({ ...receipt, paperWidth: e.target.value })}
                >
                  <option value="narrow">58 mm (thermal)</option>
                  <option value="standard">80 mm (thermal)</option>
                  <option value="a4">A4 (210 × 297 mm)</option>
                  <option value="letter">Letter (8.5 × 11 in)</option>
                  <option value="9.5x11">9.5 × 11 in (3-ply NCR / continuous)</option>
                  <option value="9.5x5.5">9.5 × 5.5 in (half-sheet continuous)</option>
                </select>
                </div>
                <div className="retail-options-field">
                <label>Footer text</label>
                <textarea
                  rows={2}
                  value={receipt.footer}
                  onChange={(e) => setReceipt({ ...receipt, footer: e.target.value })}
                  placeholder="Thank you for shopping!"
                />
              </div>
              <h4 style={{ margin: '16px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>Paper Margins (mm)</h4>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0 16px' }}>
                <div className="retail-options-field">
                  <label>Top</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginTop}
                    onChange={(e) => setReceipt({ ...receipt, marginTop: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>Bottom</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginBottom}
                    onChange={(e) => setReceipt({ ...receipt, marginBottom: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>Left</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginLeft}
                    onChange={(e) => setReceipt({ ...receipt, marginLeft: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>Right</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginRight}
                    onChange={(e) => setReceipt({ ...receipt, marginRight: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
              </div>
            </div>
              <div className="retail-options-preview" onClick={() => setShowPreview(true)} style={{ cursor: 'pointer' }}>
                <ReceiptPreview store={store} receipt={receipt} session={session} />
                <span className="retail-options-preview-hint">Click to preview</span>
              </div>
            </div>
          )}

          {activeTab === 'printer' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">Receipt Printer</h3>
              <div className="retail-options-field">
                <label>Connection</label>
                <select
                  value={hardware.printerConnection}
                  onChange={(e) => setHardware({ ...hardware, printerConnection: e.target.value })}
                >
                  <option value="auto">Auto-detect</option>
                  <option value="usb">USB</option>
                  <option value="serial">Serial (COM)</option>
                  <option value="network">Network (TCP/IP)</option>
                </select>
              </div>
              <div className="retail-options-field">
                <label>Device path</label>
                <input
                  placeholder="/dev/usb/lp0 or COM1"
                  value={hardware.printerDevicePath}
                  onChange={(e) => setHardware({ ...hardware, printerDevicePath: e.target.value })}
                />
              </div>
              <div className="retail-options-field">
                <label>Paper size</label>
                <select
                  value={hardware.printerPaperSize}
                  onChange={(e) => setHardware({ ...hardware, printerPaperSize: e.target.value })}
                >
                  <option value="58">58 mm (thermal)</option>
                  <option value="80">80 mm (thermal)</option>
                  <option value="a4">A4 (210 × 297 mm)</option>
                  <option value="letter">Letter (8.5 × 11 in)</option>
                  <option value="9.5x11">9.5 × 11 in (3-ply NCR / continuous)</option>
                  <option value="9.5x5.5">9.5 × 5.5 in (half-sheet continuous)</option>
                </select>
              </div>
              <div style={{ padding: '12px', background: '#e8e8e8', border: '1px solid #ccc', fontSize: 12, color: '#666' }}>
                Printer configuration is stored on this terminal. Changes apply after restart.
              </div>
            </div>
          )}

          {activeTab === 'scanner' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">Barcode Scanner</h3>
              {scanners.length === 0 ? (
                <div style={{ padding: 16, background: '#e8e8e8', border: '1px solid #ccc', fontSize: 13, color: '#666' }}>
                  No scanners detected. Connect a scanner and restart.
                </div>
              ) : (
                <div className="retail-options-field">
                  <label>Scanner device</label>
                  <select
                    value={hardware.scannerDeviceId}
                    onChange={(e) => setHardware({ ...hardware, scannerDeviceId: e.target.value })}
                  >
                    {scanners.map((s) => (
                      <option key={s.id} value={s.id}>{s.id}</option>
                    ))}
                  </select>
                </div>
              )}
              <div className="retail-options-field retail-options-field--row">
                <label>Auto-add product on scan</label>
                <input type="checkbox" checked disabled />
              </div>
              <div className="retail-options-field">
                <label>Input mode</label>
                <select
                  value={hardware.scannerInputMode}
                  onChange={(e) => setHardware({ ...hardware, scannerInputMode: e.target.value })}
                >
                  <option value="auto">Auto-detect</option>
                  <option value="keyboard">Keyboard wedge</option>
                  <option value="serial">Serial/COM</option>
                </select>
              </div>
            </div>
          )}

          {activeTab === 'credit' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">Credit Settings</h3>
              <div className="retail-options-field retail-options-field--row">
                <label>Enable credit sales</label>
                <input
                  type="checkbox"
                  checked={credit.enabled}
                  onChange={(e) => setCredit({ ...credit, enabled: e.target.checked })}
                />
              </div>
              {credit.enabled && (
                <>
                  <div className="retail-options-field">
                    <label>Reminder interval (hours)</label>
                    <input
                      type="number" min={1} max={720}
                      value={credit.reminderIntervalHours}
                      onChange={(e) => setCredit({ ...credit, reminderIntervalHours: Math.max(1, Number(e.target.value)) })}
                    />
                    <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                      How often the credit reminder badge appears on the POS screen
                    </span>
                  </div>
                  <div className="retail-options-field">
                    <label>Max credit limit (Rp)</label>
                    <input
                      type="number" min={0}
                      value={credit.maxLimitMinor / 100}
                      onChange={(e) => setCredit({ ...credit, maxLimitMinor: Math.max(0, Math.round(Number(e.target.value) * 100)) })}
                    />
                    <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                      Maximum outstanding balance allowed per customer (0 = unlimited)
                    </span>
                  </div>
                  <div style={{ marginTop: 16, padding: 12, background: '#e8f4e8', border: '1px solid #b0d8b0', fontSize: 12, color: '#2a6a2a' }}>
                    Credit sales are currently <strong>{credit.enabled ? 'enabled' : 'disabled'}</strong>.
                    {credit.maxLimitMinor > 0
                      ? ` Max limit: Rp ${(credit.maxLimitMinor / 100).toLocaleString('id-ID')}.`
                      : ' No limit set.'}
                  </div>
                </>
              )}
            </div>
          )}

          {activeTab === 'system' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">System</h3>
              <div className="retail-options-field">
                <label>App version</label>
                <input value="0.0.3" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label>Cashier</label>
                <input value={`${session?.display_name} (${session?.role_name})`} disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label>Terminal</label>
                  <input value="local" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
            </div>
          )}

          {/* ── Save / Close buttons ────────────── */}
          <div className="retail-options-footer">
            <button className="retail-options-btn retail-options-btn--primary" onClick={handleSave} disabled={saving}>
              {saving ? 'Saving…' : 'Save'}
            </button>
            <button className="retail-options-btn" onClick={onClose}>
              Close
            </button>
          </div>
        </div>
      </div>
      {showPreview && (
        <div className="retail-preview-overlay" onClick={() => setShowPreview(false)}>
          <div className="retail-preview-modal" onClick={(e) => e.stopPropagation()}>
            <button className="retail-preview-close" onClick={() => setShowPreview(false)}>&times;</button>
            <ReceiptPreview store={store} receipt={receipt} session={session} scale={SCALE} />
          </div>
        </div>
      )}
    </div>
  );
}
