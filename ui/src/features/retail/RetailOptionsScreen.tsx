import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { useLocalization } from '@fluent/react';
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
import { listTaxRates, type TaxRateDto } from '@/api/tax';
import { getAutoLockMinutes, setAutoLockMinutes } from '@/hooks/useIdleTimer';
import './RetailPosScreen.css';

type TabId = 'general' | 'receipt' | 'printer' | 'scanner' | 'credit' | 'system';

interface RetailOptionsScreenProps {
  onClose: () => void;
  theme?: 'light' | 'dark';
  onThemeChange?: (t: 'light' | 'dark') => void;
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
  store, receipt, session, taxRates = [],
}: {
  store: StoreSettingsDto;
  receipt: ReceiptSettingsDto;
  session: { display_name: string } | null;
  taxRates?: TaxRateDto[];
}) {
  const { l10n } = useLocalization();
  const currency = store.currency || 'IDR';
  const date = new Date().toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' });
  const time = new Date().toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' });

  const items = [
    { name: 'Indomie Goreng', qty: 2, price: 3500 },
    { name: 'Teh Botol Sosro', qty: 1, price: 5000 },
    { name: 'Nasi Goreng Spesial', qty: 1, price: 15000 },
  ];
  const subtotal = items.reduce((s, i) => s + i.price * i.qty, 0);
  // Compute tax from configured rates instead of hardcoded 11%.
  const tax = taxRates.length > 0
    ? items.reduce((totalTax, item) => {
        const lineTotal = item.price * item.qty;
        return taxRates.reduce((lineTax, rate) => {
          if (rate.is_inclusive) {
            const divisor = 10000 + rate.rate_bps;
            return lineTax + Math.round(lineTotal * rate.rate_bps / divisor);
          }
          return lineTax + Math.round(lineTotal * rate.rate_bps / 10000);
        }, totalTax);
      }, 0)
    : 0;
  const total = subtotal + tax;

  return (
    <>
      <div className="retail-receipt-header">
        <strong>{store.name || l10n.getString('settings-receipt-preview-store-fallback')}</strong>
        <small>{store.address || l10n.getString('settings-receipt-preview-address-fallback')}</small>
        <small>{date} {time}</small>
        <small>{l10n.getString('settings-receipt-preview-cashier')} {session?.display_name ?? '-'}</small>
      </div>

      <div className="retail-receipt-divider" />

      <div className="retail-receipt-items">
        <div className="retail-receipt-items-head">
          <span>{l10n.getString('settings-receipt-preview-col-item')}</span>
          <span>{l10n.getString('settings-receipt-preview-col-qty')}</span>
          <span>{l10n.getString('settings-receipt-preview-col-price')}</span>
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
          <span>{l10n.getString('settings-receipt-preview-subtotal')}</span>
          <span>{fmtPrice(subtotal, currency, receipt.showCurrency, receipt.decimalSeparator)}</span>
        </div>
        {receipt.showTax && tax > 0 && (
          <div className="retail-receipt-total-line">
            <span>{taxRates.map((r) => r.name).join(' + ') || l10n.getString('settings-receipt-preview-tax')}</span>
            <span>{fmtPrice(tax, currency, receipt.showCurrency, receipt.decimalSeparator)}</span>
          </div>
        )}
        <div className="retail-receipt-total-line retail-receipt-total-line--grand">
          <span>{l10n.getString('settings-receipt-preview-total')}</span>
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
  store, receipt, session, taxRates, scale = 1.2,
}: {
  store: StoreSettingsDto;
  receipt: ReceiptSettingsDto;
  session: { display_name: string } | null;
  taxRates: TaxRateDto[];
  scale?: number;
}) {
  const dims = getPaperDims(receipt.paperWidth);
  return (
    <div className="retail-receipt-preview" style={{
      width: dims.w * scale,
      padding: `${receipt.marginTop * scale}px ${receipt.marginRight * scale}px ${receipt.marginBottom * scale}px ${receipt.marginLeft * scale}px`,
    }}>
      <ReceiptContent store={store} receipt={receipt} session={session} taxRates={taxRates} />
    </div>
  );
}

export default function RetailOptionsScreen({ onClose, theme = 'light', onThemeChange }: RetailOptionsScreenProps) {
  const { addToast } = useToast();
  const { session } = useAuth();
  const { l10n } = useLocalization();
  const userId = session!.user_id;

  const [activeTab, setActiveTab] = useState<TabId>('general');
  const [saving, setSaving] = useState(false);
  const [showPreview, setShowPreview] = useState(false);

  // ── Tax rates (for receipt preview) ────────────────────────────

  const [taxRates, setTaxRates] = useState<TaxRateDto[]>([]);

  useEffect(() => {
    listTaxRates().then(setTaxRates).catch(() => addToast({ message: l10n.getString('settings-toast-failed-tax-rates') || 'Failed to load tax rates', type: 'error' }));
  }, []);

  // ── Store settings ────────────────────────────────────────────

  const [store, setStore] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  const [storeLoaded, setStoreLoaded] = useState(false);

  useEffect(() => {
    getStoreSettings()
      .then(setStore)
      .catch(() => addToast({ message: l10n.getString('settings-toast-failed-store-settings') || 'Failed to load store settings', type: 'error' }))
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
      .catch(() => addToast({ message: l10n.getString('settings-toast-failed-receipt-settings') || 'Failed to load receipt settings', type: 'error' }))
      .finally(() => setReceiptLoaded(true));
  }, []);

  // ── Credit settings ───────────────────────────────────────────

  const [credit, setCredit] = useState<CreditSettingsDto>({
    enabled: false, reminderIntervalHours: 24, maxLimitMinor: 0,
  });
  const [creditLoaded, setCreditLoaded] = useState(false);
  const [autoLockMinutes, setAutoLockLocal] = useState(getAutoLockMinutes);

  useEffect(() => {
    getCreditSettings()
      .then(setCredit)
      .catch(() => addToast({ message: l10n.getString('settings-toast-failed-credit-settings') || 'Failed to load credit settings', type: 'error' }))
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
      .catch(() => addToast({ message: l10n.getString('settings-toast-failed-hardware-settings') || 'Failed to load hardware settings', type: 'error' }))
      .finally(() => setHardwareLoaded(true));
  }, []);

  // ── Scanner list ──────────────────────────────────────────────

  const [scanners, setScanners] = useState<ScannerInfo[]>([]);
  useEffect(() => {
    listScanners().then(setScanners).catch(() => addToast({ message: l10n.getString('settings-toast-failed-scanners') || 'Failed to load scanners', type: 'error' }));
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
      await setStoreSettings(store, userId);
      await setReceiptSettings(receipt, userId);
      await setCreditSettings(credit, userId);
      await setHardwareSettings(hardware, userId);
      addToast({ message: l10n.getString('settings-toast-saved') || 'Settings saved', type: 'success' });
    } catch {
      addToast({ message: l10n.getString('settings-toast-failed-save') || 'Failed to save settings', type: 'error' });
    } finally {
      setSaving(false);
    }
  }, [store, receipt, credit, hardware, userId, addToast]);

  if (!storeLoaded || !receiptLoaded || !creditLoaded || !hardwareLoaded) {
    return (
      <div className="retail-pos">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', flex: 1, color: '#888', fontSize: 14 }}>
          {l10n.getString('loading')}
        </div>
      </div>
    );
  }

  return (
    <div className="retail-pos">
      {/* ── Header ──────────────────────────── */}
      <header className="retail-header">
        <div className="retail-header-store">
          <span className="retail-header-name">{l10n.getString('settings-page-title')}</span>
        </div>
        <div className="retail-header-right">
          <span className="retail-header-clock">{l10n.getString('settings-header-options')}</span>
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
              {l10n.getString(`settings-${tab.id}-tab`) || tab.label}
            </button>
          ))}
          <div style={{ flex: 1 }} />
          <button
            className="retail-options-tab retail-options-tab--danger"
            onClick={onClose}
          >
            &larr; {l10n.getString('back')}
          </button>
        </div>

        {/* Content */}
        <div className="retail-options-content">
          {activeTab === 'general' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-general-heading')}</h3>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-field-store-name')}</label>
                <input value={store.name} onChange={(e) => setStore({ ...store, name: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-field-address')}</label>
                <input value={store.address} onChange={(e) => setStore({ ...store, address: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-branch-label')}</label>
                <input value={store.branch} onChange={(e) => setStore({ ...store, branch: e.target.value })} placeholder={l10n.getString('settings-branch-placeholder')} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-tax-id-label')}</label>
                <input value={store.taxId} onChange={(e) => setStore({ ...store, taxId: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-field-default-currency')}</label>
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
                <h3 className="retail-options-heading">{l10n.getString('settings-receipt-heading')}</h3>
                <div className="retail-options-field retail-options-field--row">
                  <label>{l10n.getString('settings-show-currency-label')}</label>
                  <input
                    type="checkbox"
                    checked={receipt.showCurrency}
                    onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label>{l10n.getString('settings-show-tax-label')}</label>
                  <input
                    type="checkbox"
                    checked={receipt.showTax}
                    onChange={(e) => setReceipt({ ...receipt, showTax: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label>{l10n.getString('settings-show-table-label')}</label>
                  <input
                    type="checkbox"
                    checked={receipt.showTableNumber}
                    onChange={(e) => setReceipt({ ...receipt, showTableNumber: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-decimal-sep-label')}</label>
                  <select
                    value={receipt.decimalSeparator}
                    onChange={(e) => setReceipt({ ...receipt, decimalSeparator: e.target.value })}
                  >
                    <option value="dot">{l10n.getString('settings-decimal-sep-dot')}</option>
                    <option value="comma">{l10n.getString('settings-decimal-sep-comma')}</option>
                    <option value="none">{l10n.getString('settings-decimal-sep-none')}</option>
                  </select>
                </div>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-paper-width-label')}</label>
                <select
                  value={receipt.paperWidth}
                  onChange={(e) => setReceipt({ ...receipt, paperWidth: e.target.value })}
                >
                  <option value="narrow">{l10n.getString('settings-paper-narrow')}</option>
                  <option value="standard">{l10n.getString('settings-paper-standard')}</option>
                  <option value="a4">{l10n.getString('settings-paper-a4')}</option>
                  <option value="letter">{l10n.getString('settings-paper-letter')}</option>
                  <option value="9.5x11">{l10n.getString('settings-paper-9x11')}</option>
                  <option value="9.5x5.5">{l10n.getString('settings-paper-9x5')}</option>
                </select>
                </div>
                <div className="retail-options-field">
                <label>{l10n.getString('settings-field-footer')}</label>
                <textarea
                  rows={2}
                  value={receipt.footer}
                  onChange={(e) => setReceipt({ ...receipt, footer: e.target.value })}
                  placeholder={l10n.getString('settings-footer-placeholder')}
                />
              </div>
              <h4 style={{ margin: '16px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>{l10n.getString('settings-margins-heading')}</h4>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0 16px' }}>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-margin-top')}</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginTop}
                    onChange={(e) => setReceipt({ ...receipt, marginTop: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-margin-bottom')}</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginBottom}
                    onChange={(e) => setReceipt({ ...receipt, marginBottom: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-margin-left')}</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginLeft}
                    onChange={(e) => setReceipt({ ...receipt, marginLeft: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-margin-right')}</label>
                  <input
                    type="number" min={0} max={50}
                    value={receipt.marginRight}
                    onChange={(e) => setReceipt({ ...receipt, marginRight: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
              </div>
            </div>
              <div className="retail-options-preview" role="button" tabIndex={0} onClick={() => setShowPreview(true)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowPreview(true); } }}>
                <ReceiptPreview store={store} receipt={receipt} session={session} taxRates={taxRates} />
                <span className="retail-options-preview-hint">{l10n.getString('settings-click-preview')}</span>
              </div>
            </div>
          )}

          {activeTab === 'printer' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-printer-heading')}</h3>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-connection-label')}</label>
                <select
                  value={hardware.printerConnection}
                  onChange={(e) => setHardware({ ...hardware, printerConnection: e.target.value })}
                >
                  <option value="auto">{l10n.getString('settings-printer-connection-auto')}</option>
                  <option value="usb">{l10n.getString('settings-printer-connection-usb')}</option>
                  <option value="serial">{l10n.getString('settings-printer-connection-serial')}</option>
                  <option value="network">{l10n.getString('settings-printer-connection-network')}</option>
                </select>
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-device-path-label')}</label>
                <input
                  placeholder={l10n.getString('settings-device-path-placeholder')}
                  value={hardware.printerDevicePath}
                  onChange={(e) => setHardware({ ...hardware, printerDevicePath: e.target.value })}
                />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-printer-paper-size-label')}</label>
                <select
                  value={hardware.printerPaperSize}
                  onChange={(e) => setHardware({ ...hardware, printerPaperSize: e.target.value })}
                >
                  <option value="58">{l10n.getString('settings-paper-narrow')}</option>
                  <option value="80">{l10n.getString('settings-paper-standard')}</option>
                  <option value="a4">{l10n.getString('settings-paper-a4')}</option>
                  <option value="letter">{l10n.getString('settings-paper-letter')}</option>
                  <option value="9.5x11">{l10n.getString('settings-paper-9x11')}</option>
                  <option value="9.5x5.5">{l10n.getString('settings-paper-9x5')}</option>
                </select>
              </div>
              <div style={{ padding: '12px', background: '#e8e8e8', border: '1px solid #ccc', fontSize: 12, color: '#666' }}>
                {l10n.getString('settings-printer-info')}
              </div>
            </div>
          )}

          {activeTab === 'scanner' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-scanner-heading')}</h3>
              {scanners.length === 0 ? (
                <div style={{ padding: 16, background: '#e8e8e8', border: '1px solid #ccc', fontSize: 13, color: '#666' }}>
                  {l10n.getString('settings-scanner-none')}
                </div>
              ) : (
                <div className="retail-options-field">
                  <label>{l10n.getString('settings-scanner-device-label')}</label>
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
                <label>{l10n.getString('settings-auto-add-label')}</label>
                <input type="checkbox" checked disabled />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-input-mode-label')}</label>
                <select
                  value={hardware.scannerInputMode}
                  onChange={(e) => setHardware({ ...hardware, scannerInputMode: e.target.value })}
                >
                  <option value="auto">{l10n.getString('settings-input-mode-auto')}</option>
                  <option value="keyboard">{l10n.getString('settings-input-mode-keyboard')}</option>
                  <option value="serial">{l10n.getString('settings-input-mode-serial')}</option>
                </select>
              </div>
            </div>
          )}

          {activeTab === 'credit' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-credit-heading')}</h3>
              <div className="retail-options-field retail-options-field--row">
                <label>{l10n.getString('settings-enable-credit-label')}</label>
                <input
                  type="checkbox"
                  checked={credit.enabled}
                  onChange={(e) => setCredit({ ...credit, enabled: e.target.checked })}
                />
              </div>
              {credit.enabled && (
                <>
                  <div className="retail-options-field">
                    <label>{l10n.getString('settings-reminder-interval-label')}</label>
                    <input
                      type="number" min={1} max={720}
                      value={credit.reminderIntervalHours}
                      onChange={(e) => setCredit({ ...credit, reminderIntervalHours: Math.max(1, Number(e.target.value)) })}
                    />
                    <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                      {l10n.getString('settings-reminder-interval-hint')}
                    </span>
                  </div>
                  <div className="retail-options-field">
                    <label>{l10n.getString('settings-max-credit-label')}</label>
                    <input
                      type="number" min={0}
                      value={credit.maxLimitMinor / 100}
                      onChange={(e) => setCredit({ ...credit, maxLimitMinor: Math.max(0, Math.round(Number(e.target.value) * 100)) })}
                    />
                    <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                      {l10n.getString('settings-max-credit-hint')}
                    </span>
                  </div>
                  <div style={{ marginTop: 16, padding: 12, background: '#e8f4e8', border: '1px solid #b0d8b0', fontSize: 12, color: '#2a6a2a' }}>
                    {l10n.getString('settings-credit-status-label', { status: credit.enabled ? l10n.getString('settings-credit-status-enabled') : l10n.getString('settings-credit-status-disabled') })}
                    {credit.maxLimitMinor > 0
                      ? ` ${l10n.getString('settings-credit-status-max', { amount: (credit.maxLimitMinor / 100).toLocaleString('id-ID') })}`
                      : ` ${l10n.getString('settings-credit-status-no-limit')}`}
                  </div>
                </>
              )}
            </div>
          )}

          {activeTab === 'system' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-system-heading')}</h3>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-app-version-label')}</label>
                <input value="0.0.3" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-cashier-label')}</label>
                <input value={`${session?.display_name} (${session?.role_name})`} disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-terminal-label')}</label>
                  <input value="local" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field retail-options-field--row">
                <label>{l10n.getString('settings-theme-label')}</label>
                <select
                  value={theme}
                  onChange={(e) => onThemeChange?.(e.target.value as 'light' | 'dark')}
                  style={{ padding: '4px 8px', fontSize: 12 }}
                >
                  <option value="light">{l10n.getString('settings-theme-light')}</option>
                  <option value="dark">{l10n.getString('settings-theme-dark')}</option>
                </select>
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-auto-lock-label')}</label>
                <input
                  type="number" min={1} max={120}
                  style={{ width: 80 }}
                  value={autoLockMinutes}
                  onChange={(e) => {
                    const v = Math.max(1, Math.min(120, Number(e.target.value)));
                    setAutoLockLocal(v);
                    setAutoLockMinutes(v);
                  }}
                />
                <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                  {l10n.getString('settings-auto-lock-hint')}
                </span>
              </div>
            </div>
          )}

          {/* ── Save / Close buttons ────────────── */}
          <div className="retail-options-footer">
            <button className="retail-options-btn retail-options-btn--primary" onClick={handleSave} disabled={saving}>
              {saving ? l10n.getString('settings-saving-btn') : l10n.getString('save')}
            </button>
            <button className="retail-options-btn" onClick={onClose}>
              {l10n.getString('close')}
            </button>
          </div>
        </div>
      </div>
      {showPreview && (
        <div className="retail-preview-overlay" role="button" tabIndex={0} onClick={() => setShowPreview(false)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowPreview(false); } }}>
          <div className="retail-preview-modal" role="button" tabIndex={0} onClick={(e) => e.stopPropagation()} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); } }}>
            <button className="retail-preview-close" onClick={() => setShowPreview(false)}>&times;</button>
            <ReceiptPreview store={store} receipt={receipt} session={session} taxRates={[]} scale={SCALE} />
          </div>
        </div>
      )}
    </div>
  );
}
