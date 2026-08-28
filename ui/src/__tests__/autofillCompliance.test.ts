/**
 * Autofill Compliance Test
 *
 * Browser autofill is disallowed in the Tauri POS app — it's a desktop
 * app, not a browser, and autofill on login/PIN screens is confusing
 * and a security concern.
 *
 * Three layers of protection:
 *   1. Global: `useDisableAutofill` hook sets autocomplete="off" on all
 *      inputs via MutationObserver (catches everything, even dynamic)
 *   2. Shared Input component: adds autoComplete="off" by default
 *   3. index.html: meta tag + CSS override for webkit autofill styling
 *
 * This test verifies all three layers exist and are wired up.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

const UI_SRC = resolve(__dirname, '..');
const UI_ROOT = resolve(UI_SRC, '..');

describe('autofill is disabled globally', () => {
  it('AutofillBlocker component is rendered in App.tsx', () => {
    const app = readFileSync(resolve(UI_SRC, 'App.tsx'), 'utf-8');
    expect(app).toContain('AutofillBlocker');
    expect(app).toContain('useDisableAutofill');
  });

  it('useDisableAutofill hook exists and uses MutationObserver', () => {
    const hook = readFileSync(resolve(UI_SRC, 'hooks/useDisableAutofill.ts'), 'utf-8');
    expect(hook).toContain('MutationObserver');
    expect(hook).toContain('autocomplete');
    expect(hook).toContain("'off'");
    expect(hook).toContain('querySelectorAll');
  });

  it('shared Input component has autoComplete="off"', () => {
    const input = readFileSync(resolve(UI_SRC, 'components/Input.tsx'), 'utf-8');
    expect(input).toContain('autoComplete="off"');
  });

  it('index.html has meta autocomplete=off', () => {
    const html = readFileSync(resolve(UI_ROOT, 'index.html'), 'utf-8');
    expect(html).toContain('meta name="autocomplete" content="off"');
  });
});
