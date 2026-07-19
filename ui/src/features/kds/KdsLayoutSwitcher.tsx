import { useState, useRef, useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';
import type { KdsLayout } from '@/features/kds/hooks/useKdsPreferences';
import './KdsLayoutSwitcher.css';

interface LayoutOption {
  id: KdsLayout;
  label: string;
}

const LAYOUTS: LayoutOption[] = [
  { id: 'kanban', label: 'Kanban' },
  { id: 'focus', label: 'Focus' },
  { id: 'metro', label: 'Metro' },
];

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

export function KdsLayoutSwitcher({
  currentLayout,
  showOrderId,
  showTableNumber,
  onSelectLayout,
  onToggleOrderId,
  onToggleTableNumber,
}: KdsLayoutSwitcherProps) {
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

  const currentIcon = LAYOUTS.find((l) => l.id === currentLayout);

  return (
    <>
      <button
        ref={btnRef}
        className="kds-layout-btn"
        onClick={() => setOpen((p) => !p)}
        aria-label="Layout options"
        aria-expanded={open}
      >
        {currentIcon && <LayoutIcon layout={currentIcon.id} />}
      </button>
      {open && createPortal(
        <div
          ref={popoverRef}
          className="kds-layout-popover"
          role="dialog"
          aria-label="KDS layout and display options"
        >
          <p className="kds-layout-popover-section-title">Layout</p>
          <div className="kds-layout-options">
            {LAYOUTS.map(({ id, label }) => (
              <button
                key={id}
                className={`kds-layout-option ${id === currentLayout ? 'kds-layout-option--active' : ''}`}
                onClick={() => { onSelectLayout(id); close(); }}
                aria-label={label}
                aria-pressed={id === currentLayout}
              >
                <LayoutIcon layout={id} />
                <span>{label}</span>
              </button>
            ))}
          </div>
          <p className="kds-layout-popover-section-title">Display</p>
          <label className="kds-layout-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={showOrderId}
              onChange={(e) => onToggleOrderId(e.target.checked)}
            />
            <span className="kds-layout-toggle-label">Order ID</span>
          </label>
          <label className="kds-layout-toggle">
            <input
              type="checkbox"
              role="switch"
              checked={showTableNumber}
              onChange={(e) => onToggleTableNumber(e.target.checked)}
            />
            <span className="kds-layout-toggle-label">Table Number</span>
          </label>
        </div>,
        document.body,
      )}
    </>
  );
}
