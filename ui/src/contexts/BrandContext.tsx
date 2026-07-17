/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  type ReactNode,
} from 'react';
import { getBrandSettings, type BrandSettings } from '@/api/branding';

// ── Context value ─────────────────────────────────────────────────

interface BrandContextValue {
  /** Current brand settings (loaded from backend). */
  settings: BrandSettings;
  /** Re-fetch brand settings from the backend. */
  refreshBrandSettings: () => void;
  /** True while brand settings are being fetched on first load. */
  loading: boolean;
}

/** React context that carries brand/white-label settings. */
export const BrandContext = createContext<BrandContextValue | null>(null);

// ── Provider ──────────────────────────────────────────────────────

interface BrandProviderProps {
  children: ReactNode;
}

/**
 * Provides brand/white-label settings to the entire app.
 *
 * Loads settings from the backend on mount and exposes a
 * `refreshBrandSettings()` function so that components like
 * AppearanceSettings can trigger a re-fetch after saving.
 */
const DEFAULT_SETTINGS: BrandSettings = {
  primary_colour: '#10b981',
  logo_path: null,
  store_name: '',
};

export function BrandProvider({ children }: BrandProviderProps) {
  const [settings, setSettings] = useState<BrandSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);

  const refreshBrandSettings = useCallback(() => {
    setLoading(true);
    getBrandSettings()
      .then((s) => {
        setSettings(s);
        setLoading(false);
      })
      .catch(() => {
        setLoading(false);
        /* keep current settings on error */
      });
  }, []);

  // Load on first mount.
  useEffect(() => {
    refreshBrandSettings();
  }, [refreshBrandSettings]);

  return (
    <BrandContext.Provider value={{ settings, refreshBrandSettings, loading }}>
      {children}
    </BrandContext.Provider>
  );
}

// ── Hook ──────────────────────────────────────────────────────────

/**
 * Access the current brand settings and a refresh function.
 * Must be called within a `<BrandProvider>`.
 */
export function useBrand(): BrandContextValue {
  const ctx = useContext(BrandContext);
  if (!ctx) {
    throw new Error('useBrand must be used within a BrandProvider');
  }
  return ctx;
}

/**
 * Access brand settings safely outside of a BrandProvider (or in unit tests).
 * Returns `null` if no BrandProvider wraps the calling tree.
 */
export function useOptionalBrand(): BrandSettings | null {
  const ctx = useContext(BrandContext);
  return ctx?.settings ?? null;
}

