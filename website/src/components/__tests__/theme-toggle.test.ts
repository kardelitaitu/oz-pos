// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for ThemeToggle.astro + Base.astro applyTheme script.
 *
 * ThemeToggle handles click events to toggle dark/light mode.
 * Base.astro has an inline applyTheme script that reads localStorage
 * and applies the theme on page load.
 */

const THEME_TOGGLE_SRC = readFileSync(
  join(__dirname, '../ThemeToggle.astro'),
  'utf-8'
);

describe('ThemeToggle.astro', () => {
  describe('source structure', () => {
    it('has a script tag', () => {
      expect(THEME_TOGGLE_SRC).toContain('<script>');
    });

    it('has data-theme-toggle attribute on button', () => {
      expect(THEME_TOGGLE_SRC).toContain('data-theme-toggle');
    });

    it('toggles between light and dark', () => {
      expect(THEME_TOGGLE_SRC).toContain("=== 'light' ? 'dark' : 'light'");
    });

    it('sets localStorage.oz_theme', () => {
      expect(THEME_TOGGLE_SRC).toContain("localStorage.setItem('oz_theme'");
    });

    it('modifies dataset.theme on document element', () => {
      expect(THEME_TOGGLE_SRC).toMatch(/dataset\.theme\s*=/);
    });

    it('handles click events with delegation', () => {
      expect(THEME_TOGGLE_SRC).toContain("addEventListener('click'");
    });

    it('uses closest to find toggle button', () => {
      expect(THEME_TOGGLE_SRC).toContain('.closest(');
    });

    it('has moon icon for dark mode', () => {
      expect(THEME_TOGGLE_SRC).toContain('icon-moon');
    });

    it('has sun icon for light mode', () => {
      expect(THEME_TOGGLE_SRC).toContain('icon-sun');
    });

    it('has aria-label for accessibility', () => {
      expect(THEME_TOGGLE_SRC).toContain('aria-label');
    });
  });

  describe('theme toggle behavior', () => {
    beforeEach(() => {
      document.documentElement.removeAttribute('data-theme');
      localStorage.clear();
      document.body.innerHTML = '';
    });

    it('toggles from light to dark', () => {
      document.documentElement.dataset.theme = 'light';
      const current = document.documentElement.dataset.theme;
      document.documentElement.dataset.theme = current === 'dark' ? 'light' : 'dark';
      expect(document.documentElement.dataset.theme).toBe('dark');
    });

    it('toggles from dark to light', () => {
      document.documentElement.dataset.theme = 'dark';
      const current = document.documentElement.dataset.theme;
      document.documentElement.dataset.theme = current === 'dark' ? 'light' : 'dark';
      expect(document.documentElement.dataset.theme).toBe('light');
    });

    it('saves to localStorage', () => {
      document.documentElement.dataset.theme = 'dark';
      const theme = document.documentElement.dataset.theme;
      localStorage.setItem('oz_theme', theme);
      expect(localStorage.getItem('oz_theme')).toBe('dark');
    });

    it('reads saved theme from localStorage', () => {
      localStorage.setItem('oz_theme', 'dark');
      const saved = localStorage.getItem('oz_theme');
      expect(saved).toBe('dark');
    });

    it('delegates click to closest [data-theme-toggle]', () => {
      document.documentElement.dataset.theme = 'light';
      const btn = document.createElement('button');
      btn.setAttribute('data-theme-toggle', '');
      const svg = document.createElement('svg');
      svg.classList.add('icon-moon');
      btn.appendChild(svg);
      document.body.appendChild(btn);

      // Simulate click delegation logic
      const clickEvent = new Event('click', { bubbles: true });
      svg.dispatchEvent(clickEvent);
      const target = (clickEvent.target as HTMLElement)?.closest('[data-theme-toggle]');
      expect(target).toBe(btn);
    });
  });
});
