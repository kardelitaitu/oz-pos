// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

/**
 * Tests for ThemeToggle.astro and Base.astro theme scripts.
 *
 * ThemeToggle is an Astro component with a client <script> that toggles
 * document.documentElement.dataset.theme and persists to localStorage.
 * Base.astro has an inline script that applies the stored theme on load.
 *
 * We use the eval-based pattern (same as LocaleSwitcher) to run the real
 * scripts in vitest's context where fake timers and localStorage work.
 */

// ── ThemeToggle script (from ThemeToggle.astro <script>) ─────────────

const THEME_TOGGLE_SCRIPT = `
  // Store handler reference so tests can clean it up
  if (window.__themeHandler) {
    document.removeEventListener('click', window.__themeHandler);
  }
  window.__themeHandler = function(e) {
    var btn = (e.target).closest('[data-theme-toggle]');
    if (!btn) return;
    var html = document.documentElement;
    var next = html.dataset.theme === 'light' ? 'dark' : 'light';
    html.dataset.theme = next;
    localStorage.setItem('oz_theme', next);
  };
  document.addEventListener('click', window.__themeHandler);
`;

// ── applyTheme script (from Base.astro <script is:inline>) ──────────

const APPLY_THEME_SCRIPT = `
  (function() {
    var applyTheme = function() {
      var t = localStorage.getItem('oz_theme');
      document.documentElement.dataset.theme = t === 'dark' ? 'dark' : 'light';
    };
    applyTheme();
    document.addEventListener('astro:after-swap', applyTheme);
  })();
`;

// ── Server-rendered HTML for the toggle button ───────────────────────

function renderToggleHtml(): string {
  return (
    `<button type="button" data-theme-toggle ` +
    `class="theme-toggle" aria-label="Toggle theme" title="Toggle theme">` +
    `<svg class="icon-moon" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"></svg>` +
    `<svg class="icon-sun" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"></svg>` +
    `</button>`
  );
}

function bootThemeToggle() {
  document.body.innerHTML = renderToggleHtml();
  (0, eval)(THEME_TOGGLE_SCRIPT);
  return {
    btn: () => document.querySelector('[data-theme-toggle]') as HTMLElement,
  };
}

function bootApplyTheme() {
  (0, eval)(APPLY_THEME_SCRIPT);
}

beforeEach(() => {
  // Remove stale click listeners from previous eval'd scripts.
  // We use a proxy on addEventListener to capture handler references.
  document.body.innerHTML = '';
  document.documentElement.dataset.theme = '';
  localStorage.clear();
});

afterEach(() => {
  // Remove the click handler to prevent listener accumulation across tests
  if ((window as any).__themeHandler) {
    document.removeEventListener('click', (window as any).__themeHandler);
    delete (window as any).__themeHandler;
  }
  document.body.innerHTML = '';
  document.documentElement.dataset.theme = '';
  localStorage.clear();
});

// ── ThemeToggle HTML structure ───────────────────────────────────────

describe('ThemeToggle — HTML structure', () => {
  it('renders a button with data-theme-toggle attribute', () => {
    document.body.innerHTML = renderToggleHtml();
    const btn = document.querySelector('[data-theme-toggle]');
    expect(btn).not.toBeNull();
    expect(btn!.tagName).toBe('BUTTON');
  });

  it('has an aria-label for accessibility', () => {
    document.body.innerHTML = renderToggleHtml();
    const btn = document.querySelector('[data-theme-toggle]');
    expect(btn!.getAttribute('aria-label')).toBeTruthy();
  });

  it('contains both moon and sun SVG icons', () => {
    document.body.innerHTML = renderToggleHtml();
    expect(document.querySelector('.icon-moon')).not.toBeNull();
    expect(document.querySelector('.icon-sun')).not.toBeNull();
  });
});

// ── ThemeToggle click behavior ───────────────────────────────────────

describe('ThemeToggle — click toggling', () => {
  it('switches from dark to light on click', () => {
    document.documentElement.dataset.theme = 'dark';
    const m = bootThemeToggle();

    m.btn().click();

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('switches from light to dark on click', () => {
    document.documentElement.dataset.theme = 'light';
    const m = bootThemeToggle();

    m.btn().click();

    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('defaults to light when clicking with no initial theme', () => {
    // No theme set — dataset.theme is '' which is not 'light', so next = 'light'
    const m = bootThemeToggle();

    m.btn().click();

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('persists the new theme to localStorage', () => {
    document.documentElement.dataset.theme = 'dark';
    const m = bootThemeToggle();

    m.btn().click();

    expect(localStorage.getItem('oz_theme')).toBe('light');
  });

  it('persists dark when toggling from light', () => {
    document.documentElement.dataset.theme = 'light';
    const m = bootThemeToggle();

    m.btn().click();

    expect(localStorage.getItem('oz_theme')).toBe('dark');
  });

  it('does nothing when clicking outside the toggle button', () => {
    document.documentElement.dataset.theme = 'dark';
    document.body.innerHTML = '<div id="other">click me</div>';
    (0, eval)(THEME_TOGGLE_SCRIPT);

    document.getElementById('other')!.click();

    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(localStorage.getItem('oz_theme')).toBeNull();
  });

  it('works when clicking a child element inside the toggle', () => {
    // The SVG icons are children of the button — clicks on them should bubble up
    document.documentElement.dataset.theme = 'dark';
    document.body.innerHTML =
      `<button type="button" data-theme-toggle>` +
      `<svg class="icon-moon"><path d="M13.5 9.2A5.5 5.5 0 1 1 6.8 2.5"/></svg>` +
      `</button>`;
    (0, eval)(THEME_TOGGLE_SCRIPT);

    document.querySelector('.icon-moon')!.dispatchEvent(new MouseEvent('click', { bubbles: true }));

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('double-click toggles back to the original theme', () => {
    document.documentElement.dataset.theme = 'dark';
    const m = bootThemeToggle();

    m.btn().click();
    m.btn().click();

    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(localStorage.getItem('oz_theme')).toBe('dark');
  });
});

// ── Base.astro applyTheme initializer ────────────────────────────────

describe('applyTheme — layout initializer', () => {
  it('applies "dark" from localStorage on load', () => {
    localStorage.setItem('oz_theme', 'dark');
    bootApplyTheme();

    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('applies "light" from localStorage on load', () => {
    localStorage.setItem('oz_theme', 'light');
    bootApplyTheme();

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('defaults to "light" when localStorage has no theme', () => {
    bootApplyTheme();

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('defaults to "light" for an unrecognized value', () => {
    localStorage.setItem('oz_theme', 'auto');
    bootApplyTheme();

    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('re-applies theme on astro:after-swap event', () => {
    localStorage.setItem('oz_theme', 'dark');
    bootApplyTheme();

    // Simulate SPA navigation that changes the theme
    document.documentElement.dataset.theme = 'light';

    // Astro fires this after SPA page transitions
    document.dispatchEvent(new Event('astro:after-swap'));

    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('re-applies on astro:after-swap even if localStorage changed', () => {
    localStorage.setItem('oz_theme', 'light');
    bootApplyTheme();

    expect(document.documentElement.dataset.theme).toBe('light');

    // User changed theme in another tab
    localStorage.setItem('oz_theme', 'dark');
    document.dispatchEvent(new Event('astro:after-swap'));

    expect(document.documentElement.dataset.theme).toBe('dark');
  });
});
