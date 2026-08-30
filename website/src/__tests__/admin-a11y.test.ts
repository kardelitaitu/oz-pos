// @vitest-environment jsdom
// Admin a11y contract tests — bug hunt round 8.
// B33: login.html status messages (#error-msg / #success-msg) render with
//      no role=alert/status, so screen readers never announce login errors
//      or success (WCAG 4.1.3 Status Messages). Same for the tab panels:
//      role=tab without aria-controls and groups without role=tabpanel
//      leaves the tab pattern half-wired.
// B34: admin.js flash() appended a bare div — toasts were silent to
//      screen readers. Extracted to admin-utils.flashMessage with
//      role=alert so it is testable (admin.js itself is not importable).

import { readFileSync } from 'node:fs';
import { describe, expect, it, vi } from 'vitest';
import utils from '../../public/admin/admin-utils.js';

const loginHtml = new DOMParser().parseFromString(
  readFileSync('public/admin/login.html', 'utf8'), 'text/html');

describe('login.html status announcements (B33: WCAG 4.1.3)', () => {
  it('error container is announced assertively', () => {
    const el = loginHtml.getElementById('error-msg');
    expect(el).not.toBeNull();
    expect(el.getAttribute('role')).toBe('alert');
  });

  it('success container is announced politely', () => {
    const el = loginHtml.getElementById('success-msg');
    expect(el).not.toBeNull();
    expect(el.getAttribute('role')).toBe('status');
  });

  it('mode tabs control their panels and panels name their tabs', () => {
    // The tab pattern was half-wired: role=tab existed but nothing told
    // an AT which element holds the tab's content.
    const pairs: Array<[string, string]> = [
      ['tab-otp', 'otp-group'],
      ['tab-password', 'password-group'],
    ];
    for (const [tabId, panelId] of pairs) {
      const tab = loginHtml.getElementById(tabId);
      const panel = loginHtml.getElementById(panelId);
      expect(tab?.getAttribute('aria-controls')).toBe(panelId);
      expect(panel?.getAttribute('role')).toBe('tabpanel');
      expect(panel?.getAttribute('aria-labelledby')).toBe(tabId);
    }
  });
});

describe('admin-utils flashMessage (B34: toasts were silent to screen readers)', () => {
  it('exists and renders an announced toast in the container', () => {
    expect(typeof utils.flashMessage).toBe('function');
    const root = document.createElement('div');
    const f = utils.flashMessage(root, 'Saved');
    expect(root.children.length).toBe(1);
    expect(f.className).toContain('flash');
    expect(f.getAttribute('role')).toBe('alert');
    expect(f.textContent).toBe('Saved');
  });

  it('auto-removes after the default 3s lifetime', () => {
    vi.useFakeTimers();
    try {
      const root = document.createElement('div');
      utils.flashMessage(root, 'Bye');
      expect(root.children.length).toBe(1);
      vi.advanceTimersByTime(3000);
      expect(root.children.length).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it('honors a custom lifetime', () => {
    vi.useFakeTimers();
    try {
      const root = document.createElement('div');
      utils.flashMessage(root, 'Quick', 500);
      vi.advanceTimersByTime(499);
      expect(root.children.length).toBe(1);
      vi.advanceTimersByTime(1);
      expect(root.children.length).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });
});
