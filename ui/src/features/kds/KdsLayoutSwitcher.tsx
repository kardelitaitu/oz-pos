import { useState, useRef, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { Localized, useLocalization } from '@fluent/react';
import type { KdsLayout } from '@/features/kds/hooks/useKdsPreferences';
import './KdsLayoutSwitcher.css';

const LAYOUT_IDS: KdsLayout[] = ['kanban', 'focus', 'metro'];

/** Map layout id → l10n key for translated display name. */
const LAYOUT_KEY_MAP: Record<KdsLayout, string> = {
  kanban: 'kds-layout-kanban',
  focus: 'kds-layout-focus',
  metro: 'kds-layout-metro',
};

function LayoutIcon({ layout }: { layout: KdsLayout }) {
  return (
    <svg className="kds-layout-icon" viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {layout === 'kanban' && (
        <>
          <rect x="2" y="6" width="8" height="20" rx="1.5" />
          <rect x="12" y="6" width="8" height="20" rx="1.5" />
          <rect x="22" y="6" width="8" height="20" rx="1.5" />
        </>
      )}
      {layout === 'focus' && (
        <>
          <rect x="2" y="4" width="28" height="4" rx="1.5" />
          <rect x="2" y="12" width="28" height="4" rx="1.5" />
          <rect x="2" y="20" width="28" height="4" rx="1.5" />
        </>
      )}
      {layout === 'metro' && (
        <>
          <rect x="2" y="2" width="13" height="13" rx="1.5" />
          <rect x="17" y="2" width="13" height="13" rx="1.5" />
          <rect x="2" y="17" width="13" height="13" rx="1.5" />
          <rect x="17" y="17" width="13" height="13" rx="1.5" />
        </>
      )}
    </svg>
  );
}

interface KdsLayoutSwitcherProps {
  currentLayout: KdsLayout;
  showOrderId: boolean;
  showTableNumber: boolean;
  onSelectLayout: (layout: KdsLayout) => void;
  onToggleOrderId: (show: boolean) => void;
  onToggleTableNumber: (show: boolean) => void;
}

function layoutLabel(layout: KdsLayout): string {
  return LAYOUT_KEY_MAP[layout] || layout;
}

export function KdsLayoutSwitcher({
  currentLayout,
  showOrderId,
  showTableNumber,
  onSelectLayout,
  onToggleOrderId,
  onToggleTableNumber,
}: KdsLayoutSwitcherProps) {
  const { l10n } = useLocalization();
  const [open, setOpen] = useState(false);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setOpen(false), []);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node) &&
        btnRef.current &&
        !btnRef.current.contains(e.target as Node)
      ) {
        close();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [open, close]);

  const currentIcon = currentLayout;

  return (
    <>
      <button
        ref={btnRef}
        className="kds-layout-btn"
        onClick={() => setOpen((p) => !p)}
        aria-label={l10n.getString('kds-layout-options-aria') || 'Layout options'}
        aria-expanded={open}
      >
        {currentIcon && <LayoutIcon layout={currentIcon} />}
      </button>
      {open && createPortal(
        <div
          ref={popoverRef}
          className="kds-layout-popover"
          role="dialog"
          aria-label={l10n.getString('kds-layout-popover-aria') || 'KDS layout and display options'}
        >
          <p className="kds-layout-popover-section-title"><Localized id="kds-layout-label">Layout</Localized></p>
          <div className="kds-layout-options">
            {LAYOUT_IDS.map((id) => (
              <button
                key={id}
                className={`kds-layout-option ${id === currentLayout ? 'kds-layout-option--active' : ''}`}
                onClick={() => { onSelectLayout(id); close(); }}
                aria-label={l10n.getString(layoutLabel(id)) || id}
                aria-pressed={id === currentLayout}
              >
                <LayoutIcon layout={id} />
                <span><Localized id={layoutLabel(id)}>{id}</Localized></span>
              </button>
            ))}
          </div>
          <p className="kds-layout-popover-section-title"><Localized id="kds-layout-display-label">Display</Localized></p>
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
          <label className="kds-layout-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={showOrderId}
              onChange={(e) => onToggleOrderId(e.target.checked)}
            />
            <span className="kds-layout-toggle-label"><Localized id="kds-layout-order-id">Order ID</Localized></span>
          </label>
          {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
          <label className="kds-layout-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={showTableNumber}
              onChange={(e) => onToggleTableNumber(e.target.checked)}
            />
            <span className="kds-layout-toggle-label"><Localized id="kds-layout-table-number">Table Number</Localized></span>
          </label>
        </div>,
        document.body,
      )}
    </>
  );
}
