import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { render, screen, renderHook } from '@testing-library/react';
import { ThemeProvider, useTheme } from '@/frontend/shell/ThemeProvider';
import { BrandProvider } from '@/contexts/BrandContext';
import type { ReactNode } from 'react';

// ── Hooks only used for type-level assertions are tested implicitly
//    via renderHook + wrapper. No need to import Theme explicitly.

const STORAGE_KEY = 'oz-pos-theme-v2';

/** Shared wrapper that provides both brand and theme context. */
function Wrapper({ children }: { children: ReactNode }) {
  return (
    <BrandProvider>
      <ThemeProvider>{children}</ThemeProvider>
    </BrandProvider>
  );
}

/**
 * Reset the DOM + storage environment between tests.
 */
function resetEnvironment() {
  localStorage.clear();
  const html = document.documentElement;
  html.removeAttribute('data-theme');
  html.classList.remove('is-theme-transitioning');
}

function setMatchMedia(matches: boolean) {
  window.matchMedia = vi.fn().mockImplementation((_query: string) => ({
    matches,
    media: _query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }));
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('ThemeProvider', () => {
  beforeEach(() => {
    resetEnvironment();
    setMatchMedia(false); // light mode by default
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  // ── Rendering ──────────────────────────────────────────────────

  it('renders children', () => {
    render(
      <BrandProvider>
        <ThemeProvider>
          <div data-testid="child">Hello</div>
        </ThemeProvider>
      </BrandProvider>,
    );
    expect(screen.getByTestId('child')).toHaveTextContent('Hello');
  });

  // ── Initial theme detection ────────────────────────────────────

  it('defaults to default theme when localStorage is empty and OS is light', () => {
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('default');
  });

  it('defaults to default theme even when prefers-color-scheme is dark', () => {
    setMatchMedia(true);
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('default');
  });

  it('reads stored theme from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('dark');
  });

  it('prefers localStorage override over OS preference', () => {
    localStorage.setItem(STORAGE_KEY, 'light');
    setMatchMedia(true); // OS says dark, but localStorage says light
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('light');
  });

  // ── toggleTheme ────────────────────────────────────────────────

  it('toggleTheme switches from default to light', () => {
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('default');
    act(() => result.current.toggleTheme());
    expect(result.current.theme).toBe('light');
  });

  it('toggleTheme switches from light to dark', () => {
    localStorage.setItem(STORAGE_KEY, 'light');
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('light');
    act(() => result.current.toggleTheme());
    expect(result.current.theme).toBe('dark');
  });

  it('toggleTheme switches from dark to default', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(result.current.theme).toBe('dark');
    act(() => result.current.toggleTheme());
    expect(result.current.theme).toBe('default');
  });

  // ── setTheme ───────────────────────────────────────────────────

  it('setTheme sets a specific theme', () => {
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    act(() => result.current.setTheme('dark'));
    expect(result.current.theme).toBe('dark');
  });

  it('setTheme can switch from dark to light', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    act(() => result.current.setTheme('light'));
    expect(result.current.theme).toBe('light');
  });

  // ── DOM side-effects ───────────────────────────────────────────

  it('removes data-theme attribute on html element for default theme', () => {
    const html = document.documentElement;
    render(
      <BrandProvider>
        <ThemeProvider><div /></ThemeProvider>
      </BrandProvider>,
    );
    expect(html.hasAttribute('data-theme')).toBe(false);
  });

  it('updates data-theme when theme changes', () => {
    const html = document.documentElement;
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    act(() => result.current.toggleTheme()); // default -> light
    expect(html.getAttribute('data-theme')).toBe('light');
  });

  it('persists theme to localStorage', () => {
    const { result } = renderHook(() => useTheme(), { wrapper: Wrapper });
    act(() => result.current.setTheme('dark'));
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });

  it('adds is-theme-transitioning class on mount and removes it after timeout', () => {
    vi.useFakeTimers();
    const html = document.documentElement;

    // On first render the initial useEffect runs and sets the class.
    renderHook(() => useTheme(), { wrapper: Wrapper });
    expect(html.classList.contains('is-theme-transitioning')).toBe(true);

    // Advance past the 300ms timeout.
    act(() => vi.advanceTimersByTime(300));
    expect(html.classList.contains('is-theme-transitioning')).toBe(false);
    vi.useRealTimers();
  });

  // ── useTheme errors ────────────────────────────────────────────

  it('useTheme throws when used outside ThemeProvider', () => {
    // Suppress console.error for the expected React error boundary output.
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => renderHook(() => useTheme())).toThrow(
      'useTheme must be used within a ThemeProvider',
    );
    consoleSpy.mockRestore();
  });
});
