import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  useCallback,
  type ReactNode,
} from 'react';
import { useBrand } from '@/contexts/BrandContext';
import { deriveAccentPalette, applyAccentPalette } from '@/utils/color';

// ── Types ──────────────────────────────────────────────────────────

export type Theme = 'light' | 'dark';

interface ThemeContextValue {
  /** Current resolved theme. */
  theme: Theme;
  /** Toggle between light and dark. */
  toggleTheme: () => void;
  /** Set a specific theme. */
  setTheme: (t: Theme) => void;
}

// ── Context ────────────────────────────────────────────────────────

const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'oz-pos-theme';

// ── Provider ───────────────────────────────────────────────────────

interface ThemeProviderProps {
  children: ReactNode;
}

/**
 * Provides the active theme and a toggle function to the component
 * tree. On first render it respects:
 * 1. `localStorage` (manual override persisted across sessions)
 * 2. `prefers-color-scheme` (OS-level preference)
 *
 * Sets `data-theme` on `<html>` so the CSS dark-mode selector works.
 * Also reactively applies the brand accent palette from BrandContext
 * whenever the primary colour changes.
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(() => {
    // 1. Check localStorage
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') return stored;

    // 2. Fall back to OS preference
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
    return 'light';
  });

  // Sync `data-theme` attribute and localStorage whenever theme changes.
  // Also applies a brief transitioning class so CSS can animate the switch.
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    const html = document.documentElement;

    // Add transitioning class to animate the theme change.
    html.classList.add('is-theme-transitioning');
    html.setAttribute('data-theme', theme);
    localStorage.setItem(STORAGE_KEY, theme);

    // Remove the class after transitions complete so subsequent
    // color changes (hover, focus) don't animate.
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => {
      html.classList.remove('is-theme-transitioning');
      timeoutRef.current = null;
    }, 300);

    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [theme]);

  // Listen for OS-level preference changes when no manual override is set.
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      // Only auto-switch if the user hasn't explicitly chosen.
      if (!localStorage.getItem(STORAGE_KEY)) {
        setThemeState(e.matches ? 'dark' : 'light');
      }
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  // Reactively apply brand accent palette whenever brand settings change.
  const { settings: brandSettings } = useBrand();
  useEffect(() => {
    if (brandSettings.primary_colour) {
      const palette = deriveAccentPalette(brandSettings.primary_colour);
      applyAccentPalette(palette);
    }
  }, [brandSettings.primary_colour]);

  const toggleTheme = useCallback(() => {
    setThemeState((prev) => (prev === 'light' ? 'dark' : 'light'));
  }, []);

  const setTheme = useCallback((t: Theme) => {
    setThemeState(t);
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, toggleTheme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

// ── Hook ───────────────────────────────────────────────────────────

/**
 * Access the current theme and toggle function.
 * Must be called within a `<ThemeProvider>`.
 */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return ctx;
}
