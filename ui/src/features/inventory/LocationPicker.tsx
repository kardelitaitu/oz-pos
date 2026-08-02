import { useState, useEffect, useCallback, useRef, memo } from 'react';
import { useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { requiredLocalized } from '@/frontend/shared';
import { listInventoryLocations, type InventoryLocation } from '@/api/inventory';
import './LocationPicker.css';

// ── Location type label mapping (LOC-05) ───────────────────────────
// Every supported type maps to a value-bearing Fluent key; unknown future
// types fall back to `loc-type-unknown` so the dropdown never shows raw
// machine values (e.g. `warehouse`, `transit`) in any locale.
const LOCATION_TYPE_KEYS: Record<InventoryLocation['type'], string> = {
  store: 'loc-type-store',
  warehouse: 'loc-type-warehouse',
  transit: 'loc-type-transit',
  damaged: 'loc-type-damaged',
  virtual: 'loc-type-virtual',
};

function locationTypeLabel(l10n: ReturnType<typeof useLocalization>['l10n'], type: string): string {
  const key = LOCATION_TYPE_KEYS[type as InventoryLocation['type']] ?? 'loc-type-unknown';
  return requiredLocalized(l10n, key);
}

interface LocationPickerProps {
  /** Currently selected location ID. */
  value: string;
  /** Called when the user selects a location. */
  onChange: (locationId: string, locationName: string) => void;
  /** Label for the dropdown trigger (defaults to the localized 'Location'). */
  label?: string;
}

/**
 * LocationPicker — dropdown for selecting an inventory location.
 *
 * Loads active inventory locations from the backend on mount and displays
 * them in a dropdown that follows the same pattern as StoreSwitcher.
 * Used in the inventory workspace header to filter views by location.
 */
const LocationPicker = memo(function LocationPicker({
  value,
  onChange,
  label,
}: LocationPickerProps) {
  const { sessionToken } = useWorkspace();
  const { l10n } = useLocalization();
  const l10nRef = useRef(l10n);
  l10nRef.current = l10n;
  const token = sessionToken ?? '';

  const [locations, setLocations] = useState<InventoryLocation[]>([]);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [activeIndex, setActiveIndex] = useState(-1);
  const ref = useRef<HTMLDivElement>(null);
  const listboxRef = useRef<HTMLUListElement>(null);

  // ── Load locations ────────────────────────────────────────────────

  const load = useCallback(async () => {
    if (!token) {
      setLoading(false);
      return;
    }
    setLoading(true);
    setLoadError(null);
    try {
      const data = await listInventoryLocations(token);
      setLocations(data.filter((loc) => loc.is_active));
    } catch {
      // Durable error state (INV-08): surface a retry affordance instead
      // of silently rendering nothing when the locations fetch fails.
      setLoadError(requiredLocalized(l10nRef.current, 'loc-picker-error-load'));
    } finally {
      setLoading(false);
    }
  }, [token]); // l10n via ref — stable dep chain

  useEffect(() => {
    load();
  }, [load]);

  // ── Click outside to close ──────────────────────────────────────

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // ── Selection ──────────────────────────────────────────────────

  const handleSelect = useCallback(
    (location: InventoryLocation) => {
      if (location.id !== value) {
        onChange(location.id, location.name);
      }
      setOpen(false);
    },
    [onChange, value],
  );

  // ── Keyboard: full listbox navigation (LOC-04) ──────────────
  // ArrowUp/Down move the active descendant (wrapping), Home/End jump to the
  // first/last option, Enter/Space select the active option, Escape closes and
  // restores focus to the trigger. Focus moves to the listbox while open so
  // `aria-activedescendant` is announced by screen readers.

  useEffect(() => {
    if (!open) return;
    listboxRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open || activeIndex < 0 || !listboxRef.current) return;
    const optionEl = listboxRef.current.querySelector(`[data-index="${activeIndex}"]`);
    // jsdom lacks scrollIntoView — guard the call so tests (and old browsers)
    // don't crash on the navigation effect.
    optionEl?.scrollIntoView?.({ block: 'nearest' });
  }, [open, activeIndex]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (locations.length === 0) return;
      switch (e.key) {
        case 'Escape':
          setOpen(false);
          setActiveIndex(-1);
          ref.current?.querySelector<HTMLButtonElement>('.location-picker-trigger')?.focus();
          break;
        case 'ArrowDown':
          e.preventDefault();
          setActiveIndex((i) => (i + 1) % locations.length);
          break;
        case 'ArrowUp':
          e.preventDefault();
          setActiveIndex((i) => (i <= 0 ? locations.length - 1 : i - 1));
          break;
        case 'Home':
          e.preventDefault();
          setActiveIndex(0);
          break;
        case 'End':
          e.preventDefault();
          setActiveIndex(locations.length - 1);
          break;
        case 'Enter':
        case ' ': {
          e.preventDefault();
          if (activeIndex >= 0 && locations[activeIndex]) {
            handleSelect(locations[activeIndex]);
          }
          break;
        }
        default:
          break;
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, locations, activeIndex, handleSelect]);

  // ── Find current location name ─────────────────────────────────

  const currentLocation = locations.find((loc) => loc.id === value);
  const fallbackLabel = label ?? requiredLocalized(l10n, 'loc-picker-label');
  const currentName = currentLocation?.name ?? fallbackLabel;

  // ── Render ─────────────────────────────────────────────────────

  if (loading) return null;

  if (loadError) {
    return (
      <div className="location-picker location-picker-error" role="alert">
        <span className="location-picker-error-text">{loadError}</span>
        <button
          type="button"
          className="location-picker-retry"
          onClick={load}
          aria-label={requiredLocalized(l10n, 'retry')}
        >
          {requiredLocalized(l10n, 'retry')}
        </button>
      </div>
    );
  }

  if (locations.length === 0) return null;

  return (
    <div className="location-picker" ref={ref}>
      <button
        type="button"
        className="location-picker-trigger"
        onClick={() => {
          const next = !open;
          if (next) {
            const currentIdx = locations.findIndex((loc) => loc.id === value);
            setActiveIndex(currentIdx >= 0 ? currentIdx : 0);
          } else {
            setActiveIndex(-1);
          }
          setOpen(next);
        }}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls="location-picker-listbox"
        aria-label={requiredLocalized(l10n, 'loc-picker-trigger-aria', { name: currentName })}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
          <circle cx="12" cy="10" r="3" />
        </svg>
        <span className="location-picker-name">{currentName}</span>
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
          className={`location-picker-chevron ${open ? 'location-picker-chevron--open' : ''}`}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {open && (
        <ul
          id="location-picker-listbox"
          ref={listboxRef}
          className="location-picker-dropdown"
          role="listbox"
          tabIndex={-1}
          aria-label={requiredLocalized(l10n, 'loc-picker-listbox-aria')}
          aria-activedescendant={
            activeIndex >= 0 && locations[activeIndex]
              ? `location-picker-option-${locations[activeIndex].id}`
              : undefined
          }
        >
          {locations.map((loc, idx) => (
            <li key={loc.id} role="none">
              <button
                type="button"
                role="option"
                id={`location-picker-option-${loc.id}`}
                data-index={idx}
                aria-selected={loc.id === value}
                className={`location-picker-option ${loc.id === value ? 'location-picker-option--active' : ''} ${activeIndex === idx ? 'location-picker-option--highlighted' : ''}`}
                onClick={() => handleSelect(loc)}
              >
                <span className="location-picker-option-name">{loc.name}</span>
                <span className="location-picker-option-meta">
                  {locationTypeLabel(l10n, loc.type)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
});

export default LocationPicker;
