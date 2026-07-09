import { useState, useEffect, useCallback, useRef } from 'react';
import { useLocalization } from '@fluent/react';
import { listStores, setPrimaryStore, type StoreProfile } from '@/api/stores';
import './StoreSwitcher.css';

export default function StoreSwitcher() {
  const { l10n } = useLocalization();
  const [stores, setStores] = useState<StoreProfile[]>([]);
  const [primary, setPrimary] = useState<StoreProfile | null>(null);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const ref = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    try {
      const data = await listStores();
      setStores(data);
      const p = data.find((s) => s.is_primary) ?? data[0] ?? null;
      setPrimary(p);
    } catch {
      // silently fail
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleSelect = useCallback(async (store: StoreProfile) => {
    if (store.id === primary?.id) {
      setOpen(false);
      return;
    }
    try {
      await setPrimaryStore(store.id);
      setPrimary(store);
      setStores((prev) =>
        prev.map((s) => ({ ...s, is_primary: s.id === store.id })),
      );
    } catch {
      // silently fail
    }
    setOpen(false);
  }, [primary]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  if (loading || stores.length <= 1) return null;

  const currentName = primary?.name ?? l10n.getString('store-switcher-select');

  return (
    <div className="store-switcher" ref={ref}>
      <button
        type="button"
        className="store-switcher-trigger"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={l10n.getString('store-switcher-current-aria', { name: currentName })}
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
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
        <span className="store-switcher-name">{currentName}</span>
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
          className={`store-switcher-chevron ${open ? 'store-switcher-chevron--open' : ''}`}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {open && (
        <ul className="store-switcher-dropdown" role="listbox" aria-label={l10n.getString('store-switcher-list-aria')}>
          {stores.map((store) => (
            <li key={store.id} role="none">
              <button
                type="button"
                role="option"
                aria-selected={store.id === primary?.id}
                className={`store-switcher-option ${store.id === primary?.id ? 'store-switcher-option--active' : ''}`}
                onClick={() => handleSelect(store)}
              >
                <span className="store-switcher-option-name">{store.name}</span>
                <span className="store-switcher-option-meta">
                  {store.currency}
                  {store.is_primary ? l10n.getString('store-switcher-primary') : ''}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
