import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import { ThemeProvider } from '@/components/ThemeProvider';
import ThemeToggle from '@/components/ThemeToggle';

// ── Wrapper ──────────────────────────────────────────────────────────

function wrap(children: React.ReactNode) {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(
    new FluentResource('theme-toggle-label = Toggle theme\n'),
  );
  const l10n = new ReactLocalization([bundle]);
  return (
    <LocalizationProvider l10n={l10n}>
      <ThemeProvider>{children}</ThemeProvider>
    </LocalizationProvider>
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

  it('renders a moon icon in light mode', () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    const svgs = button.querySelectorAll('svg');
    expect(svgs.length).toBe(1);
    // Moon icon has a <path> but no <circle>.
    expect(svgs[0]?.querySelector('path')).toBeInTheDocument();
    expect(svgs[0]?.querySelector('circle')).toBeNull();
  });

  it('renders a sun icon in dark mode', () => {
    localStorage.setItem('oz-pos-theme', 'dark');
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
    // Initially light → aria-label says "Switch to dark mode".
    expect(button).toHaveAttribute('aria-label', 'Switch to dark mode');
  });

  it('aria-label updates after toggling theme', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');
    expect(button).toHaveAttribute('aria-label', 'Switch to dark mode');

    await userEvent.click(button);

    expect(button).toHaveAttribute('aria-label', 'Switch to light mode');
  });

  // ── Interaction ────────────────────────────────────────────────

  it('clicking toggles the theme from light to dark', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');

    await userEvent.click(button);

    // After toggling to dark, the sun icon (circle) should appear.
    const svg = button.querySelector('svg');
    expect(svg?.querySelector('circle')).toBeInTheDocument();
  });

  it('clicking twice toggles back to light', async () => {
    render(wrap(<ThemeToggle />));
    const button = screen.getByTestId('theme-toggle');

    await userEvent.click(button); // light → dark
    await userEvent.click(button); // dark → light

    // Back to light mode: moon icon (path only, no circle).
    const svg = button.querySelector('svg');
    expect(svg?.querySelector('path')).toBeInTheDocument();
    expect(svg?.querySelector('circle')).toBeNull();
  });
});
