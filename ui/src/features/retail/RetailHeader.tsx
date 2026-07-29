import { useLocalization } from '@fluent/react';
import { formatMoney } from '@/types/domain';
import type { StoreSettingsDto } from '@/api/settings';
import type { ShiftDto } from '@/api/shifts';

interface RetailHeaderProps {
  storeSettings: StoreSettingsDto;
  shiftLoading: boolean;
  activeShift: ShiftDto | null;
  displayName: string;
  dateStr: string;
  timeStr: string;
  shiftDuration: string | null;
  onWorkspacePicker: () => void;
}

/** Retail POS header — store info, shift badge, cashier, clock. */
export default function RetailHeader({
  storeSettings,
  shiftLoading,
  activeShift,
  displayName,
  dateStr,
  timeStr,
  shiftDuration,
  onWorkspacePicker,
}: RetailHeaderProps) {
  const { l10n } = useLocalization();

  return (
    <header className="retail-header">
      <div className="retail-header-store">
        {storeSettings.logo && (
          <img
            src={`data:image/png;base64,${storeSettings.logo}`}
            alt={storeSettings.name || l10n.getString('retail-store-logo-alt') || 'Store logo'}
            className="retail-header-logo"
            style={{ height: 32, marginRight: 8 }}
          />
        )}
        <div>
          <span className="retail-header-name">{storeSettings.name || l10n.getString('retail-store-name-fallback')}</span>
          {storeSettings.branch && <span className="retail-header-branch"> &middot; {storeSettings.branch}</span>}
          <span className="retail-header-address">{storeSettings.address || ''}</span>
        </div>
      </div>
      <div className="retail-header-right">
        {shiftLoading ? (
          <span className="retail-shift-badge">{l10n.getString('loading')}</span>
        ) : activeShift ? (
          <span className="retail-shift-badge">
            {l10n.getString('retail-shift-label')} &middot; {formatMoney({ minor_units: activeShift.totalSalesMinor, currency: storeSettings.currency })}
          </span>
        ) : (
          <span className="retail-shift-badge" style={{ opacity: 0.6 }}>{l10n.getString('retail-no-shift')}</span>
        )}
        <button
          type="button"
          className="retail-header-nav-btn"
          onClick={onWorkspacePicker}
          title={l10n.getString('retail-header-workspaces-title') || 'Back to workspaces'}
          aria-label={l10n.getString('retail-header-workspaces-aria') || 'Back to workspaces'}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
            <rect x="2" y="3" width="22" height="14" rx="2" ry="2" />
            <line x1="8" y1="21" x2="16" y2="21" />
            <line x1="12" y1="17" x2="12" y2="21" />
          </svg>
        </button>
        <div className="retail-header-cashier">
          <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
            <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
          </svg>
          <span>{displayName}</span>
        </div>
        <span className="retail-header-clock">
          <span className="retail-header-date">{dateStr}</span>
          <span>{timeStr}</span>
          {shiftDuration && <span className="retail-header-duration">{shiftDuration}</span>}
        </span>
      </div>
    </header>
  );
}
