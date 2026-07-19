import { useState, useEffect, useCallback, useRef } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listInventoryLocations, type InventoryLocation } from '@/api/inventory';
import './LocationPicker.css';

interface LocationPickerProps {
  /** Currently selected location ID. */
  value: string;
  /** Called when the user selects a location. */
  onChange: (locationId: string, locationName: string) => void;
  /** Label for the dropdown trigger (default: 'Location'). */
  label?: string;
}

/**
 * LocationPicker — dropdown for selecting an inventory location.
 *
 * Loads active inventory locations from the backend on mount and displays
 * them in a dropdown that follows the same pattern as StoreSwitcher.
 * Used in the inventory workspace header to filter views by location.
 */
export default function LocationPicker({
  value,
  onChange,
  label = 'Location',
}: LocationPickerProps) {
  const { session } = useAuth();
  const { sessionToken } = useWorkspace();
  const token = sessionToken ?? session?.session_token ?? '';

  const [locations, setLocations] = useState<InventoryLocation[]>([]);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const ref = useRef<HTMLDivElement>(null);

  // ── Load locations ────────────────────────────────────────────────

  const load = useCallback(async () => {
    if (!token) {
      setLoading(false);
      return;
    }
    try {
      const data = await listInventoryLocations(token);
      setLocations(data.filter((loc) => loc.is_active));
    } catch {
      // silently fail
    } finally {
      setLoading(false);
    }
  }, [token]);

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

  // ── Keyboard: close on Escape ─────────────────────────────────

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        ref.current?.querySelector('button')?.focus();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open]);

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

  // ── Find current location name ─────────────────────────────────

  const currentLocation = locations.find((loc) => loc.id === value);
  const currentName = currentLocation?.name ?? label;

  // ── Render ─────────────────────────────────────────────────────

  if (loading || locations.length === 0) return null;

  return (
    <div className="location-picker" ref={ref}>
      <button
        type="button"
        className="location-picker-trigger"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`Select inventory location. Current: ${currentName}`}
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
          className="location-picker-dropdown"
          role="listbox"
          aria-label="Inventory locations"
        >
          {locations.map((loc) => (
            <li key={loc.id} role="none">
              <button
                type="button"
                role="option"
                aria-selected={loc.id === value}
                className={`location-picker-option ${loc.id === value ? 'location-picker-option--active' : ''}`}
                onClick={() => handleSelect(loc)}
              >
                <span className="location-picker-option-name">{loc.name}</span>
                <span className="location-picker-option-meta">
                  {loc.type}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
