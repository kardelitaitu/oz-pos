import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { withFluent } from '@/locales/test-utils';

import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { BrandProvider } from '@/contexts/BrandContext';
import ThemeToggle from '@/frontend/shell/ThemeToggle';

// ── Wrapper ──────────────────────────────────────────────────────────

function wrap(children: React.ReactNode) {
  return withFluent(
    <BrandProvider>
      <ThemeProvider>{children}</ThemeProvider>
    </BrandProvider>,
  );
}

// ── Tests ─────────────────────────────────────────────────────────────

describe('ThemeToggle', () => {
  beforeEach(() => {
    localStorage.clear();
    const html = document.documentElement;
    html.removeAttribute('data-theme');
    html.classList.remove('is-theme-transitioning');

    window.matchMedia = vi.fn().mockImplementation((_query: string) => ({
      matches: false,
      media: _query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ── Rendering ──────────────────────────────────────────────────

  it('renders a button with theme-toggle testid', () => {
    render(wrap(<ThemeToggle />));
    expect(screen.getByTestId('theme-toggle')).toBeInTheDocument();
    expect(screen.getByTestId('theme-toggle').tagName).toBe('BUTTON');
  });

  it('renders a sun icon in default mode (to switch to light)', () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    const svgs = button.querySelectorAll('svg');
    expect(svgs.length).toBe(1);
    expect(svgs[0]?.querySelector('circle')).toBeInTheDocument();
  });

  it('renders a sun icon in dark mode', () => {
    localStorage.setItem('oz-pos-theme-v2', 'dark');
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    const svgs = button.querySelectorAll('svg');
    expect(svgs.length).toBe(1);
    // Sun icon has a <circle> element.
    expect(svgs[0]?.querySelector('circle')).toBeInTheDocument();
  });

  // ── Accessibility ──────────────────────────────────────────────

  it('has an aria-label that reflects current theme', () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    // Asserts on the user-visible substring (and SR-announced string),
    // not on Fluent's internal bidi-isolating marks U+2068/U+2069.
    // Production's `getBundle()` passes `useIsolating: false`, so the
    // aria-label is the literal plain string — no markers.
    expect(button).toHaveAttribute(
      'aria-label',
      expect.stringContaining('Switch to light mode'),
    );
  });

  it('aria-label updates after toggling theme', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    expect(button).toHaveAttribute(
      'aria-label',
      expect.stringContaining('Switch to light mode'),
    );

    await userEvent.click(button); // default -> light

    expect(button).toHaveAttribute(
      'aria-label',
      expect.stringContaining('Switch to dark mode'),
    );
  });

  // ── Interaction ────────────────────────────────────────────────

  it('clicking toggles the theme from default to light', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');

    await userEvent.click(button);

    // After toggling to light, the moon icon (path only) should appear.
    const svg = button.querySelector('svg');
    expect(svg?.querySelector('path')).toBeInTheDocument();
    expect(svg?.querySelector('circle')).toBeNull();
  });

  it('clicking twice toggles to dark', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');

    await userEvent.click(button); // default → light
    await userEvent.click(button); // light → dark

    // In dark mode: sun icon (circle).
    const svg = button.querySelector('svg');
    expect(svg?.querySelector('circle')).toBeInTheDocument();
  });
});
