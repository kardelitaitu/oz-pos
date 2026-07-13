import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { useLocalization } from '@fluent/react';
import { invoke } from '@tauri-apps/api/core';
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
import { getGatewayStatus, type GatewayStatus } from '@/api/gateway';
import { listScanners, listDisplays, type ScannerInfo } from '@/api/hardware';
import { listTaxRates, type TaxRateDto } from '@/api/tax';
import { getAutoLockMinutes, setAutoLockMinutes } from '@/hooks/useIdleTimer';
import { useCloudSync } from '@/hooks/useCloudSync';
import { LanguageSelector } from '@/i18n/LanguageSelector';
import { AppearanceSettings } from '@/features/settings/AppearanceSettings';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';
import DataManagementScreen from '@/features/settings/DataManagementScreen';
import './RetailPosScreen.css';

type TabId = 'general' | 'receipt' | 'printer' | 'scanner' | 'credit' | 'payments' | 'system' | 'appearance' | 'features' | 'data' | 'sync';

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
  { id: 'payments', label: 'Payments' },
  { id: 'system', label: 'System' },
  { id: 'appearance', label: 'Appearance' },
  { id: 'features', label: 'Features' },
  { id: 'data', label: 'Data' },
  { id: 'sync', label: 'Sync' },
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

/** Retail options / settings screen — multi-tab configuration panel for general, receipt, printer, scanner, credit, payments, system, appearance, features, data, and sync settings. */
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
    let mounted = true;
    listTaxRates().then((rates) => { if (mounted) setTaxRates(rates); }).catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-tax-rates') || 'Failed to load tax rates', type: 'error' }); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Store settings ────────────────────────────────────────────

  const [store, setStore] = useState<StoreSettingsDto>({ name: '', address: '', taxId: '', currency: 'IDR', branch: '', logo: '' });
  const [storeLoaded, setStoreLoaded] = useState(false);

  useEffect(() => {
    let mounted = true;
    getStoreSettings()
      .then((s) => { if (mounted) setStore(s); })
      .catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-store-settings') || 'Failed to load store settings', type: 'error' }); })
      .finally(() => { if (mounted) setStoreLoaded(true); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Receipt settings ──────────────────────────────────────────

  const [receipt, setReceipt] = useState<ReceiptSettingsDto>({
    showCurrency: true, decimalSeparator: 'dot', showTax: true,
    footer: '', paperWidth: 'standard', showTableNumber: false,
    marginTop: 0, marginBottom: 0, marginLeft: 0, marginRight: 0,
  });
  const [receiptLoaded, setReceiptLoaded] = useState(false);

  useEffect(() => {
    let mounted = true;
    getReceiptSettings()
      .then((r) => { if (mounted) setReceipt(r); })
      .catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-receipt-settings') || 'Failed to load receipt settings', type: 'error' }); })
      .finally(() => { if (mounted) setReceiptLoaded(true); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Credit settings ───────────────────────────────────────────

  const [credit, setCredit] = useState<CreditSettingsDto>({
    enabled: false, reminderIntervalHours: 24, maxLimitMinor: 0,
  });
  const [creditLoaded, setCreditLoaded] = useState(false);
  const [autoLockMinutes, setAutoLockLocal] = useState(getAutoLockMinutes);

  useEffect(() => {
    let mounted = true;
    getCreditSettings()
      .then((c) => { if (mounted) setCredit(c); })
      .catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-credit-settings') || 'Failed to load credit settings', type: 'error' }); })
      .finally(() => { if (mounted) setCreditLoaded(true); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

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
    let mounted = true;
    getHardwareSettings()
      .then((h) => { if (mounted) setHardware(h); })
      .catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-hardware-settings') || 'Failed to load hardware settings', type: 'error' }); })
      .finally(() => { if (mounted) setHardwareLoaded(true); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Scanner list ──────────────────────────────────────────────

  const [scanners, setScanners] = useState<ScannerInfo[]>([]);
  useEffect(() => {
    let mounted = true;
    listScanners().then((s) => { if (mounted) setScanners(s); }).catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-scanners') || 'Failed to load scanners', type: 'error' }); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Payment gateway config ─────────────────────────────────────

  const [gateways, setGateways] = useState<GatewayStatus[]>([]);
  const [stripeKey, setStripeKey] = useState('');
  const [squareKey, setSquareKey] = useState('');
  const [midtransKey, setMidtransKey] = useState('');
  useEffect(() => {
    let mounted = true;
    getGatewayStatus()
      .then((statuses) => {
        if (!mounted) return;
        setGateways(Array.isArray(statuses) ? statuses : []);
      })
      .catch(() => {
        if (!mounted) return;
        // Treat failure as no gateways so the UI shows the fallback message.
        setGateways([]);
      });
    // Load existing keys from secure DB-backed settings
    (async () => {
      try {
        const sk: string | null = await invoke('get_setting', { key: 'stripe.api_key' });
        const sq: string | null = await invoke('get_setting', { key: 'square.api_key' });
        const mt: string | null = await invoke('get_setting', { key: 'midtrans.server_key' });
        if (!mounted) return;
        if (sk) setStripeKey(sk);
        if (sq) setSquareKey(sq);
        if (mt) setMidtransKey(mt);
      } catch { /* ignore — settings DB may not be available yet */ }
    })();
    return () => { mounted = false; };
  }, []);

  // ── Quick tender presets ───────────────────────────────────────

  const [tenderPresets, setTenderPresets] = useState<number[]>(() => {
    try {
      const saved = localStorage.getItem('retail-tender-presets');
      if (saved) return JSON.parse(saved) as number[];
    } catch { /* ignore */ }
    return [5000, 10000, 20000, 50000, 100000];
  });

  // ── Sound toggle ───────────────────────────────────────────────

  const [soundEnabled, setSoundEnabledLocal] = useState(() => {
    try {
      return localStorage.getItem('retail-sound-enabled') !== 'false';
    } catch {
      return true;
    }
  });

  // ── Cloud sync ────────────────────────────────────────────────

  const { persist: persistSync, ...sync } = useCloudSync({ addToast, l10n });

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

  // ── Customer display ────────────────────────────────────────

  const [displays, setDisplays] = useState<string[]>([]);
  const [displayTestMsg, setDisplayTestMsg] = useState('');

  useEffect(() => {
    let mounted = true;
    listDisplays().then((d) => { if (mounted) setDisplays(d); }).catch(() => { if (mounted) addToast({ message: l10n.getString('settings-toast-failed-displays') || 'Failed to load displays', type: 'error' }); });
    return () => { mounted = false; };
  }, [addToast, l10n]);

  // ── Save ──────────────────────────────────────────────────────

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await setStoreSettings(store, userId);
      await setReceiptSettings(receipt, userId);
      await setCreditSettings(credit, userId);
      await setHardwareSettings(hardware, userId);

      // Save payment gateway keys via secure settings IPC
      try {
        if (stripeKey) await invoke('set_setting', { key: 'stripe.api_key', value: stripeKey, user_id: userId });
        else await invoke('set_setting', { key: 'stripe.api_key', value: '', user_id: userId });
        if (squareKey) await invoke('set_setting', { key: 'square.api_key', value: squareKey, user_id: userId });
        else await invoke('set_setting', { key: 'square.api_key', value: '', user_id: userId });
        if (midtransKey) await invoke('set_setting', { key: 'midtrans.server_key', value: midtransKey, user_id: userId });
        else await invoke('set_setting', { key: 'midtrans.server_key', value: '', user_id: userId });
      } catch { /* ignore — settings DB may not be available */ }

      // Save cloud sync settings (non-secret config to localStorage,
      // auth token to the secure DB-backed setting channel).
      await persistSync(userId);

      // Save tender presets to localStorage
      localStorage.setItem('retail-tender-presets', JSON.stringify(tenderPresets));
      // Save sound preference
      localStorage.setItem('retail-sound-enabled', String(soundEnabled));

      addToast({ message: l10n.getString('settings-toast-saved') || 'Settings saved', type: 'success' });
    } catch {
      addToast({ message: l10n.getString('settings-toast-failed-save') || 'Failed to save settings', type: 'error' });
    } finally {
      setSaving(false);
    }
  }, [store, receipt, credit, hardware, stripeKey, squareKey, midtransKey, tenderPresets, soundEnabled, persistSync, userId, addToast, l10n]);

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
                <label htmlFor="general-store-name">{l10n.getString('settings-field-store-name')}</label>
                <input id="general-store-name" value={store.name} onChange={(e) => setStore({ ...store, name: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="general-address">{l10n.getString('settings-field-address')}</label>
                <input id="general-address" value={store.address} onChange={(e) => setStore({ ...store, address: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="general-branch">{l10n.getString('settings-branch-label')}</label>
                <input id="general-branch" value={store.branch} onChange={(e) => setStore({ ...store, branch: e.target.value })} placeholder={l10n.getString('settings-branch-placeholder')} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="general-tax-id">{l10n.getString('settings-tax-id-label')}</label>
                <input id="general-tax-id" value={store.taxId} onChange={(e) => setStore({ ...store, taxId: e.target.value })} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="general-default-currency">{l10n.getString('settings-field-default-currency')}</label>
                <select id="general-default-currency" value={store.currency} onChange={(e) => setStore({ ...store, currency: e.target.value })}>
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
                  <label htmlFor="receipt-show-currency">{l10n.getString('settings-show-currency-label')}</label>
                  <input
                    id="receipt-show-currency"
                    type="checkbox"
                    checked={receipt.showCurrency}
                    onChange={(e) => setReceipt({ ...receipt, showCurrency: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label htmlFor="receipt-show-tax">{l10n.getString('settings-show-tax-label')}</label>
                  <input
                    id="receipt-show-tax"
                    type="checkbox"
                    checked={receipt.showTax}
                    onChange={(e) => setReceipt({ ...receipt, showTax: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field retail-options-field--row">
                  <label htmlFor="receipt-show-table">{l10n.getString('settings-show-table-label')}</label>
                  <input
                    id="receipt-show-table"
                    type="checkbox"
                    checked={receipt.showTableNumber}
                    onChange={(e) => setReceipt({ ...receipt, showTableNumber: e.target.checked })}
                  />
                </div>
                <div className="retail-options-field">
                  <label htmlFor="receipt-decimal-sep">{l10n.getString('settings-decimal-sep-label')}</label>
                  <select
                    id="receipt-decimal-sep"
                    value={receipt.decimalSeparator}
                    onChange={(e) => setReceipt({ ...receipt, decimalSeparator: e.target.value })}
                  >
                    <option value="dot">{l10n.getString('settings-decimal-sep-dot')}</option>
                    <option value="comma">{l10n.getString('settings-decimal-sep-comma')}</option>
                    <option value="none">{l10n.getString('settings-decimal-sep-none')}</option>
                  </select>
                </div>
                <div className="retail-options-field">
                  <label htmlFor="receipt-paper-width">{l10n.getString('settings-paper-width-label')}</label>
                <select
                  id="receipt-paper-width"
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
                <label htmlFor="receipt-footer">{l10n.getString('settings-field-footer')}</label>
                <textarea
                  id="receipt-footer"
                  rows={2}
                  value={receipt.footer}
                  onChange={(e) => setReceipt({ ...receipt, footer: e.target.value })}
                  placeholder={l10n.getString('settings-footer-placeholder')}
                />
              </div>
              <h4 style={{ margin: '16px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>{l10n.getString('settings-margins-heading')}</h4>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0 16px' }}>
                <div className="retail-options-field">
                  <label htmlFor="receipt-margin-top">{l10n.getString('settings-margin-top')}</label>
                  <input
                    id="receipt-margin-top"
                    type="number" min={0} max={50}
                    value={receipt.marginTop}
                    onChange={(e) => setReceipt({ ...receipt, marginTop: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label htmlFor="receipt-margin-bottom">{l10n.getString('settings-margin-bottom')}</label>
                  <input
                    id="receipt-margin-bottom"
                    type="number" min={0} max={50}
                    value={receipt.marginBottom}
                    onChange={(e) => setReceipt({ ...receipt, marginBottom: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label htmlFor="receipt-margin-left">{l10n.getString('settings-margin-left')}</label>
                  <input
                    id="receipt-margin-left"
                    type="number" min={0} max={50}
                    value={receipt.marginLeft}
                    onChange={(e) => setReceipt({ ...receipt, marginLeft: Math.max(0, Number(e.target.value)) })}
                  />
                </div>
                <div className="retail-options-field">
                  <label htmlFor="receipt-margin-right">{l10n.getString('settings-margin-right')}</label>
                  <input
                    id="receipt-margin-right"
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
                <label htmlFor="printer-connection">{l10n.getString('settings-connection-label')}</label>
                <select
                  id="printer-connection"
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
                <label htmlFor="printer-device-path">{l10n.getString('settings-device-path-label')}</label>
                <input
                  id="printer-device-path"
                  placeholder={l10n.getString('settings-device-path-placeholder')}
                  value={hardware.printerDevicePath}
                  onChange={(e) => setHardware({ ...hardware, printerDevicePath: e.target.value })}
                />
              </div>
              <div className="retail-options-field">
                <label htmlFor="printer-paper-size">{l10n.getString('settings-printer-paper-size-label')}</label>
                <select
                  id="printer-paper-size"
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
                  <label htmlFor="scanner-device">{l10n.getString('settings-scanner-device-label')}</label>
                  <select
                    id="scanner-device"
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
                <label htmlFor="scanner-auto-add">{l10n.getString('settings-auto-add-label')}</label>
                <input id="scanner-auto-add" type="checkbox" checked disabled />
              </div>
              <div className="retail-options-field">
                <label htmlFor="scanner-input-mode">{l10n.getString('settings-input-mode-label')}</label>
                <select
                  id="scanner-input-mode"
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
                <label htmlFor="credit-enabled">{l10n.getString('settings-enable-credit-label')}</label>
                <input
                  id="credit-enabled"
                  type="checkbox"
                  checked={credit.enabled}
                  onChange={(e) => setCredit({ ...credit, enabled: e.target.checked })}
                />
              </div>
              {credit.enabled && (
                <>
                  <div className="retail-options-field">
                    <label htmlFor="credit-reminder-interval">{l10n.getString('settings-reminder-interval-label')}</label>
                    <input
                      id="credit-reminder-interval"
                      type="number" min={1} max={720}
                      value={credit.reminderIntervalHours}
                      onChange={(e) => setCredit({ ...credit, reminderIntervalHours: Math.max(1, Number(e.target.value)) })}
                    />
                    <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                      {l10n.getString('settings-reminder-interval-hint')}
                    </span>
                  </div>
                  <div className="retail-options-field">
                    <label htmlFor="credit-max-credit">{l10n.getString('settings-max-credit-label')}</label>
                    <input
                      id="credit-max-credit"
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

          {activeTab === 'payments' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-payments-heading') || 'Payment Gateways'}</h3>

              {/* ── Gateway status badges ────────── */}
              <div style={{ display: 'flex', gap: 8, marginBottom: 12, flexWrap: 'wrap' }}>
                {gateways.length === 0 ? (
                  <span style={{ fontSize: 12, color: '#888' }}>{l10n.getString('settings-payments-no-gateways') || 'No payment gateways configured'}</span>
                ) : (
                  gateways.map((g) => (
                    <span
                      key={g.name}
                      style={{
                        display: 'inline-flex', alignItems: 'center', gap: 4,
                        padding: '4px 10px', borderRadius: 4, fontSize: 12,
                        background: g.configured ? '#e8f4e8' : '#f8e8e8',
                        color: g.configured ? '#2a6a2a' : '#8a2a2a',
                        border: `1px solid ${g.configured ? '#b0d8b0' : '#d8b0b0'}`,
                      }}
                    >
                      <span
                        style={{
                          width: 8, height: 8, borderRadius: '50%',
                          background: g.configured && g.online ? '#2a6a2a' : g.configured ? '#d8a030' : '#8a2a2a',
                          display: 'inline-block',
                        }}
                      />
                      {g.name}
                    </span>
                  ))
                )}
              </div>

              {/* ── Stripe ──────────────────────── */}
              <details style={{ marginBottom: 12 }}>
                <summary style={{ cursor: 'pointer', fontSize: 13, fontWeight: 600, padding: '4px 0' }} aria-label="Stripe">
                  💳 Stripe
                </summary>
                <div className="retail-options-field" style={{ marginTop: 8 }}>
                  <label htmlFor="payments-stripe-key">{l10n.getString('settings-stripe-api-key') || 'Stripe API Key'}</label>
                  <input
                    id="payments-stripe-key"
                    type="password"
                    placeholder={l10n.getString('settings-stripe-key-placeholder') || 'sk_live_...'}
                    value={stripeKey}
                    onChange={(e) => setStripeKey(e.target.value)}
                  />
                  <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                    {l10n.getString('settings-stripe-key-hint') || 'Enter your Stripe secret key to enable card payments'}
                  </span>
                </div>
              </details>

              {/* ── Square ─────────────────────── */}
              <details style={{ marginBottom: 12 }}>
                <summary style={{ cursor: 'pointer', fontSize: 13, fontWeight: 600, padding: '4px 0' }} aria-label="Square">
                  🟦 Square
                </summary>
                <div className="retail-options-field" style={{ marginTop: 8 }}>
                  <label htmlFor="payments-square-key">{l10n.getString('settings-square-api-key') || 'Square API Key'}</label>
                  <input
                    id="payments-square-key"
                    type="password"
                    placeholder={l10n.getString('settings-square-key-placeholder') || 'sq0atp-...'}
                    value={squareKey}
                    onChange={(e) => setSquareKey(e.target.value)}
                  />
                  <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                    {l10n.getString('settings-square-key-hint') || 'Enter your Square access token to enable card payments'}
                  </span>
                </div>
              </details>

              {/* ── QRIS (Midtrans) ────────────── */}
              <details style={{ marginBottom: 12 }}>
                <summary style={{ cursor: 'pointer', fontSize: 13, fontWeight: 600, padding: '4px 0' }} aria-label="QRIS Midtrans">
                  📱 QRIS (Midtrans)
                </summary>
                <div className="retail-options-field" style={{ marginTop: 8 }}>
                  <label htmlFor="payments-midtrans-key">{l10n.getString('settings-midtrans-key') || 'Midtrans Server Key'}</label>
                  <input
                    id="payments-midtrans-key"
                    type="password"
                    placeholder={l10n.getString('settings-midtrans-key-placeholder') || 'Mid-server-...'}
                    value={midtransKey}
                    onChange={(e) => setMidtransKey(e.target.value)}
                  />
                  <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                    {l10n.getString('settings-midtrans-key-hint') || 'Enter your Midtrans server key for QRIS payments'}
                  </span>
                </div>
              </details>

              {/* ── Quick tender presets ───────── */}
              <h4 style={{ margin: '20px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>
                {l10n.getString('settings-tender-presets-heading') || 'Quick Cash Tender Buttons'}
              </h4>
              <p style={{ fontSize: 12, color: '#666', margin: '0 0 8px' }}>
                {l10n.getString('settings-tender-presets-desc') || 'Customize the quick tender buttons shown on the cash payment screen. Values are in rupiah (e.g., 50000 = Rp 50,000).'}
              </p>
              {tenderPresets.map((val, idx) => (
                <div key={idx} className="retail-options-field" style={{ marginBottom: 4 }}>
                  <label htmlFor={`payments-tender-preset-${idx + 1}`}>
                    {l10n.getString('settings-tender-preset-label', { n: idx + 1 }) ?? `Preset ${idx + 1}`}
                  </label>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                    <input
                      id={`payments-tender-preset-${idx + 1}`}
                      type="number"
                      min={0}
                      step={100}
                      style={{ width: 120 }}
                      value={val}
                      onChange={(e) => {
                        const v = Math.max(0, Math.round(Number(e.target.value) / 100) * 100);
                        setTenderPresets((prev) => prev.map((p, i) => (i === idx ? v : p)));
                      }}
                    />
                    <span style={{ fontSize: 12, color: '#888' }}>
                      {store.currency} {(val).toLocaleString('id-ID')}
                    </span>
                    <button
                      type="button"
                      onClick={() => setTenderPresets((prev) => prev.filter((_, i) => i !== idx))}
                      disabled={tenderPresets.length <= 2}
                      style={{
                        padding: '2px 6px', fontSize: 12, background: 'none',
                        border: '1px solid #ccc', cursor: tenderPresets.length <= 2 ? 'not-allowed' : 'pointer',
                        opacity: tenderPresets.length <= 2 ? 0.4 : 1,
                      }}
                      aria-label={`Remove preset ${idx + 1}`}
                    >
                      &times;
                    </button>
                  </div>
                </div>
              ))}
              <button
                type="button"
                onClick={() => setTenderPresets((prev) => [...prev, 0])}
                disabled={tenderPresets.length >= 8}
                style={{
                  marginTop: 4, padding: '4px 12px', fontSize: 12, cursor: tenderPresets.length >= 8 ? 'not-allowed' : 'pointer',
                  background: '#1a3a5c', color: '#fff', border: 'none',
                  opacity: tenderPresets.length >= 8 ? 0.4 : 1,
                }}
              >
                + {l10n.getString('settings-tender-preset-add') || 'Add preset'}
              </button>
            </div>
          )}

          {activeTab === 'system' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-system-heading')}</h3>
              <div className="retail-options-field">
                <label htmlFor="system-app-version">{l10n.getString('settings-app-version-label')}</label>
                <input id="system-app-version" value="0.0.6" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="system-cashier">{l10n.getString('settings-cashier-label')}</label>
                <input id="system-cashier" value={`${session?.display_name} (${session?.role_name})`} disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field">
                <label htmlFor="system-terminal">{l10n.getString('settings-terminal-label')}</label>
                  <input id="system-terminal" value="local" disabled style={{ background: '#e8e8e8', color: '#888' }} />
              </div>
              <div className="retail-options-field retail-options-field--row">
                <label htmlFor="system-theme">{l10n.getString('settings-theme-label')}</label>
                <select
                  id="system-theme"
                  value={theme}
                  onChange={(e) => onThemeChange?.(e.target.value as 'light' | 'dark')}
                  style={{ padding: '4px 8px', fontSize: 12 }}
                >
                  <option value="light">{l10n.getString('settings-theme-light')}</option>
                  <option value="dark">{l10n.getString('settings-theme-dark')}</option>
                </select>
              </div>
              <div className="retail-options-field retail-options-field--row">
                <label htmlFor="system-sound">{l10n.getString('settings-sound-label') || 'Sound Effects'}</label>
                <input
                  id="system-sound"
                  type="checkbox"
                  checked={soundEnabled}
                  onChange={(e) => setSoundEnabledLocal(e.target.checked)}
                />
              </div>
              <div className="retail-options-field">
                <label>{l10n.getString('settings-language-label') || 'Language'}</label>
                <LanguageSelector />
              </div>
              <div className="retail-options-field">
                <label htmlFor="system-auto-lock">{l10n.getString('settings-auto-lock-label')}</label>
                <input
                  id="system-auto-lock"
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

              {/* ── Quick links to other configuration screens ─── */}
              <h4 style={{ margin: '20px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>
                {l10n.getString('settings-quick-links-heading') || 'More Configuration'}
              </h4>
              <div style={{ padding: '8px 12px', background: '#f0f0f0', border: '1px solid #ddd', fontSize: 12, color: '#666', lineHeight: 1.5 }}>
                {l10n.getString('settings-quick-links-note') || 'Tax rates and feature toggles can be configured from the main Settings page, accessible via the sidebar.'}
              </div>

              {/* ── Customer Display ──────────────────────────────── */}
              <h4 style={{ margin: '20px 0 8px', fontSize: 12, textTransform: 'uppercase', color: '#555' }}>
                {l10n.getString('settings-display-heading') || 'Customer-Facing Display'}
              </h4>
              {displays.length === 0 ? (
                <div style={{ padding: 12, background: '#f5f5f5', border: '1px solid #ddd', fontSize: 12, color: '#888' }}>
                  {l10n.getString('settings-display-none') || 'No pole displays detected. Connect a customer-facing display to enable this feature.'}
                </div>
              ) : (
                <>
                  <p style={{ fontSize: 12, color: '#666', margin: '0 0 8px' }}>
                    {l10n.getString('settings-display-count', { count: displays.length }) || `${displays.length} display(s) connected`}
                  </p>
                  <div className="retail-options-field">
                    <label htmlFor="system-display-test">{l10n.getString('settings-display-test-label') || 'Test Message'}</label>
                    <div style={{ display: 'flex', gap: 8 }}>
                      <input
                        id="system-display-test"
                        type="text"
                        style={{ flex: 1 }}
                        placeholder={l10n.getString('settings-display-test-placeholder') || 'Welcome to our store!'}
                        value={displayTestMsg}
                        onChange={(e) => setDisplayTestMsg(e.target.value)}
                      />
                      <button
                        type="button"
                        onClick={async () => {
                          if (!displayTestMsg.trim()) return;
                          try {
                            const { displayShow } = await import('@/api/hardware');
                            await displayShow({
                              displayId: displays[0]!,
                              line1: displayTestMsg,
                              line2: '',
                            });
                            addToast({ message: l10n.getString('settings-display-test-sent') || 'Message sent to display', type: 'success' });
                          } catch {
                            addToast({ message: l10n.getString('settings-display-test-failed') || 'Failed to send to display', type: 'error' });
                          }
                        }}
                        disabled={!displayTestMsg.trim()}
                        style={{
                          padding: '4px 12px', fontSize: 11, background: '#1a3a5c', color: '#fff',
                          border: 'none', cursor: displayTestMsg.trim() ? 'pointer' : 'not-allowed',
                          opacity: displayTestMsg.trim() ? 1 : 0.5,
                        }}
                      >
                        {l10n.getString('settings-display-test-btn') || 'Show'}
                      </button>
                    </div>
                  </div>
                  <div style={{ padding: '8px 12px', background: '#f0f0f0', border: '1px solid #ddd', fontSize: 11, color: '#888', marginTop: 8 }}>
                    {l10n.getString('settings-display-info') || 'The customer-facing display shows item names and totals as they are scanned during a sale.'}
                  </div>
                </>
              )}
            </div>
          )}

          {activeTab === 'appearance' && (
            <div className="retail-options-section retail-options-section--full">
              <AppearanceSettings />
            </div>
          )}

          {activeTab === 'features' && (
            <div className="retail-options-section retail-options-section--full">
              <FeatureToggleScreen />
            </div>
          )}

          {activeTab === 'data' && (
            <div className="retail-options-section retail-options-section--full">
              <DataManagementScreen />
            </div>
          )}

          {activeTab === 'sync' && (
            <div className="retail-options-section">
              <h3 className="retail-options-heading">{l10n.getString('settings-sync-heading') || 'Cloud Sync'}</h3>

              <div className="retail-options-field retail-options-field--row">
                <label htmlFor="sync-enabled">{l10n.getString('settings-sync-enabled-label') || 'Enable cloud sync'}</label>
                <input
                  id="sync-enabled"
                  type="checkbox"
                  checked={sync.enabled}
                  onChange={(e) => sync.setEnabled(e.target.checked)}
                  disabled={!sync.serverURL.trim()}
                />
              </div>

              <div className="retail-options-field">
                <label htmlFor="sync-server-url">{l10n.getString('settings-sync-server-label') || 'Server URL'}</label>
                <input
                  id="sync-server-url"
                  type="url"
                  placeholder={l10n.getString('settings-sync-server-placeholder') || 'https://sync.oz-pos.example.com'}
                  value={sync.serverURL}
                  onChange={(e) => sync.setServerURL(e.target.value)}
                />
                <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                  {l10n.getString('settings-sync-server-hint') || 'The endpoint that receives your encrypted backup snapshots'}
                </span>
              </div>

              <div className="retail-options-field">
                <label htmlFor="sync-auth-token">{l10n.getString('settings-sync-token-label') || 'Authentication Token'}</label>
                <input
                  id="sync-auth-token"
                  type="password"
                  autoComplete="off"
                  placeholder={l10n.getString('settings-sync-token-placeholder') || 'paste sync token here'}
                  value={sync.token}
                  onChange={(e) => sync.setToken(e.target.value)}
                />
                <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                  {l10n.getString('settings-sync-token-hint') || 'Stored securely in the database — never in localStorage'}
                </span>
              </div>

              <div className="retail-options-field">
                <label htmlFor="sync-auto-interval">{l10n.getString('settings-sync-interval-label') || 'Auto-sync interval (minutes)'}</label>
                <input
                  id="sync-auto-interval"
                  type="number"
                  min={0}
                  max={1440}
                  style={{ width: 80 }}
                  value={sync.autoMinutes}
                  onChange={(e) => sync.setAutoMinutes(Math.max(0, Math.min(1440, Number(e.target.value))))}
                />
                <span style={{ fontSize: 11, color: '#888', display: 'block', marginTop: 2 }}>
                  {l10n.getString('settings-sync-interval-hint') || 'Set to 0 to disable automatic sync'}
                </span>
              </div>

              <div className="retail-options-field retail-options-field--row" style={{ gap: 8 }}>
                <button
                  type="button"
                  onClick={sync.testConnection}
                  disabled={sync.syncing || sync.pulling || !sync.serverURL.trim()}
                  className="retail-options-btn"
                >
                  {l10n.getString('settings-sync-test-connection-btn') || 'Test connection'}
                </button>
                <button
                  type="button"
                  onClick={sync.syncNow}
                  disabled={sync.syncing || sync.pulling || !sync.serverURL.trim()}
                  className="retail-options-btn retail-options-btn--primary"
                >
                  {sync.syncing
                    ? (l10n.getString('settings-sync-testing-btn') || 'Testing…')
                    : (l10n.getString('settings-sync-now-btn') || 'Sync now')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const message =
                      l10n.getString('settings-sync-confirm-overwrite') ||
                      'Overwrite local data with the server snapshot?';
                    if (window.confirm(message)) {
                      void sync.pullFromServer();
                    }
                  }}
                  disabled={sync.syncing || sync.pulling || !sync.serverURL.trim()}
                  className="retail-options-btn"
                  data-testid="sync-pull-btn"
                >
                  {sync.pulling
                    ? (l10n.getString('settings-sync-pulling-btn') || 'Pulling…')
                    : (l10n.getString('settings-sync-force-pull-btn') || 'Pull from server')}
                </button>
              </div>

              {/* ── Status & last sync ─────────── */}
              <div style={{ marginTop: 16, padding: 12, background: sync.status === 'online' ? '#e8f4e8' : '#f8e8e8', border: `1px solid ${sync.status === 'online' ? '#b0d8b0' : '#d8b0b0'}`, fontSize: 12 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                  <span style={{
                    width: 10, height: 10, borderRadius: '50%',
                    background: sync.status === 'online' ? '#2a6a2a' : '#8a2a2a',
                    display: 'inline-block',
                  }} />
                  <strong>
                    {sync.status === 'online'
                      ? (l10n.getString('settings-sync-status-online') || 'Online')
                      : (l10n.getString('settings-sync-status-offline') || 'Offline')}
                  </strong>
                </div>
                <div>
                  {l10n.getString('settings-sync-last') || 'Last sync'}:{' '}
                  {sync.lastAt ?? (l10n.getString('settings-sync-status-never') || 'Never synced')}
                </div>
                <div>
                  {l10n.getString('settings-sync-pending') || 'Pending changes'}: {sync.pending}
                </div>
              </div>

              {!sync.tokenLoaded && (
                <div style={{ marginTop: 8, fontSize: 11, color: '#888' }}>
                  {l10n.getString('loading') || 'Loading…'}
                </div>
              )}
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
        <div
          className="retail-preview-overlay"
          role="button"
          tabIndex={0}
          onClick={(e) => { if (e.target === e.currentTarget) setShowPreview(false); }}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowPreview(false); } }}
        >
          <div className="retail-preview-modal" role="dialog" aria-modal="true" aria-label={l10n.getString('settings-receipt-heading')}>
            <button className="retail-preview-close" onClick={() => setShowPreview(false)}>&times;</button>
            <ReceiptPreview store={store} receipt={receipt} session={session} taxRates={[]} scale={SCALE} />
          </div>
        </div>
      )}
    </div>
  );
}
