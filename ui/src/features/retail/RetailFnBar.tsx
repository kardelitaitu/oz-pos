import type { RefObject } from 'react';
import { useLocalization } from '@fluent/react';
import { useFeatures, FEATURES } from '@/hooks/useFeatures';

interface RetailFnBarProps {
  linesLength: number;
  heldCartId: string | null;
  activeShift: boolean;
  onPay: () => void;
  onRequestClear: () => void;
  onShowDiscount: () => void;
  onHoldResume: () => void;
  onShowSalesHistory: () => void;
  onShowCustomerSearch: () => void;
  onShowStockInquiry: () => void;
  onToggleShift: () => void;
  onOpenSettings: () => void;
  onShowQuickReturn: () => void;
  onShowTables: () => void;
  onNavigateKds: (() => void) | undefined;
  skuInputRef: RefObject<HTMLInputElement | null>;
}

/** Function key bar (F1–F12) for the retail POS terminal. Pure presentational — all callbacks are wired in the parent. */
export default function RetailFnBar({
  linesLength,
  heldCartId,
  activeShift,
  onPay,
  onRequestClear,
  onShowDiscount,
  onHoldResume,
  onShowSalesHistory,
  onShowCustomerSearch,
  onShowStockInquiry,
  onToggleShift,
  onOpenSettings,
  onShowQuickReturn,
  onShowTables,
  onNavigateKds,
  skuInputRef,
}: RetailFnBarProps) {
  const { l10n } = useLocalization();
  const { isEnabled } = useFeatures();

  return (
    <div className="retail-fn-bar" role="toolbar" aria-label={l10n.getString('retail-fn-bar-aria') || 'Function bar'}>
      <button type="button" className="retail-fn-btn" onClick={onPay} disabled={linesLength === 0}>
        <span className="retail-fn-key">F1</span> {l10n.getString('sale-pay-button')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onRequestClear} disabled={linesLength === 0}>
        <span className="retail-fn-key">F2</span> {l10n.getString('retail-fn-void')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onShowDiscount} disabled={linesLength === 0}>
        <span className="retail-fn-key">F3</span> {l10n.getString('retail-fn-diskon')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onHoldResume} disabled={!heldCartId && linesLength === 0}>
        <span className="retail-fn-key">F4</span> {heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={() => skuInputRef.current?.focus()}>
        <span className="retail-fn-key">F5</span> {l10n.getString('retail-fn-cari')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onShowSalesHistory}>
        <span className="retail-fn-key">F6</span> {l10n.getString('retail-fn-history')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onShowCustomerSearch}>
        <span className="retail-fn-key">F7</span> {l10n.getString('retail-fn-pelanggan')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onShowStockInquiry}>
        <span className="retail-fn-key">F8</span> {l10n.getString('retail-fn-stok')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onToggleShift}>
        <span className="retail-fn-key">F9</span> {activeShift ? l10n.getString('pos-shift-close-btn') : l10n.getString('pos-shift-open-btn')} {l10n.getString('retail-fn-shift')}
      </button>
      <button type="button" className="retail-fn-btn" onClick={onOpenSettings}>
        <span className="retail-fn-key">F10</span> {l10n.getString('retail-fn-options')}
      </button>
      {isEnabled(FEATURES.QUICK_RETURN) && (
        <button type="button" className="retail-fn-btn" onClick={onShowQuickReturn}>
          {l10n.getString('retail-fn-quick-return') || 'Quick Return'}
        </button>
      )}
      <button type="button" className="retail-fn-btn" onClick={onNavigateKds}>
        <span className="retail-fn-key">F12</span> {l10n.getString('kds-title') || 'KDS'}
      </button>
      {isEnabled(FEATURES.TABLE_MANAGEMENT) && (
        <button type="button" className="retail-fn-btn" onClick={onShowTables}>
          🪑 {l10n.getString('tables-title') || 'Tables'}
        </button>
      )}
    </div>
  );
}
