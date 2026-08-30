// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for the Download page segmented-trial callout script.
 *
 * The download page has a 12-line inline script that reads `?v=` from the URL
 * and conditionally shows a trial callout:
 *   - Pro trial for `kafe`/`restoran`/`restaurant`/`cafe`
 *   - Enterprise trial for `enterprise_referral`
 *
 * If this logic breaks, the wrong trial offer appears — or no offer at all —
 * for users coming from vertical landing pages. This is conversion-critical.
 */

const DOWNLOAD_SRC = readFileSync(
  join(import.meta.dirname, '..', 'pages', '[locale]', 'download.astro'),
  'utf-8',
);

// Extract the inline script from the page source
function extractScript(): string {
  // Match between <script is:inline> and </script> — multiline with indentation
  const scriptMatch = DOWNLOAD_SRC.match(/<script is:inline>([\s\S]*?)<\/script>/);
  if (!scriptMatch) throw new Error('Could not extract inline script from download.astro');
  return scriptMatch[1].trim();
}

function setupTrialCallout(): HTMLElement {
  const el = document.createElement('p');
  el.id = 'trial-callout';
  el.dataset.pro = 'Pro trial text';
  el.dataset.enterprise = 'Enterprise trial text';
  el.classList.add('hidden');
  document.body.appendChild(el);
  return el;
}

function setUrlSearch(search: string): void {
  // jsdom doesn't update window.location.search directly, so we mock it
  Object.defineProperty(window, 'location', {
    value: new URL(`https://example.com/download${search}`, 'https://example.com'),
    writable: true,
  });
}

// ─── Source structure tests ──────────────────────────────────────────

describe('Download page trial callout source', () => {
  it('has the trial-callout element with data-pro and data-enterprise', () => {
    expect(DOWNLOAD_SRC).toContain('id="trial-callout"');
    expect(DOWNLOAD_SRC).toContain('data-pro=');
    expect(DOWNLOAD_SRC).toContain('data-enterprise=');
  });

  it('has the inline script that reads URL params', () => {
    expect(DOWNLOAD_SRC).toContain('new URLSearchParams(window.location.search)');
  });

  it('maps kafe/restoran/restaurant/cafe to Pro trial', () => {
    expect(DOWNLOAD_SRC).toContain("'kafe'");
    expect(DOWNLOAD_SRC).toContain("'restoran'");
    expect(DOWNLOAD_SRC).toContain("'restaurant'");
    expect(DOWNLOAD_SRC).toContain("'cafe'");
  });

  it('maps enterprise_referral to Enterprise trial', () => {
    expect(DOWNLOAD_SRC).toContain("'enterprise_referral'");
  });

  it('starts hidden and removes hidden class when triggered', () => {
    expect(DOWNLOAD_SRC).toContain('class="mx-auto mt-6 hidden');
    expect(DOWNLOAD_SRC).toContain("classList.remove('hidden')");
  });
});

// ─── Script behavior tests ───────────────────────────────────────────

describe('Trial callout script behavior', () => {
  const SCRIPT = extractScript();

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('does nothing when no trial-callout element exists', () => {
    expect(() => eval(SCRIPT)).not.toThrow();
  });

  it('stays hidden when no ?v= param is present', () => {
    setUrlSearch('');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('');
    expect(el.classList.contains('hidden')).toBe(true);
  });

  it('stays hidden for unrecognized vertical', () => {
    setUrlSearch('?v=grocery');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('');
    expect(el.classList.contains('hidden')).toBe(true);
  });

  it('shows Pro trial text for ?v=kafe', () => {
    setUrlSearch('?v=kafe');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('shows Pro trial text for ?v=restoran', () => {
    setUrlSearch('?v=restoran');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('shows Pro trial text for ?v=restaurant', () => {
    setUrlSearch('?v=restaurant');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('shows Pro trial text for ?v=cafe', () => {
    setUrlSearch('?v=cafe');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('shows Enterprise trial text for ?v=enterprise_referral', () => {
    setUrlSearch('?v=enterprise_referral');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Enterprise trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('is case-insensitive for vertical param', () => {
    setUrlSearch('?v=Kafe');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('trims whitespace from vertical param', () => {
    setUrlSearch('?v=%20kafe%20');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });

  it('ignores other URL params alongside v', () => {
    setUrlSearch('?v=kafe&ref=homepage');
    const el = setupTrialCallout();
    eval(SCRIPT);
    expect(el.textContent).toBe('Pro trial text');
    expect(el.classList.contains('hidden')).toBe(false);
  });
});

// ─── i18n key tests ──────────────────────────────────────────────────

describe('Download trial i18n keys', () => {
  const enJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', 'i18n', 'en.json'), 'utf-8'),
  );
  const idJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', 'i18n', 'id.json'), 'utf-8'),
  );

  it('en has trialCallout key', () => {
    expect(enJson.download?.trialCallout).toBeTruthy();
  });

  it('en has trialCalloutEnterprise key', () => {
    expect(enJson.download?.trialCalloutEnterprise).toBeTruthy();
  });

  it('id has trialCallout key', () => {
    expect(idJson.download?.trialCallout).toBeTruthy();
  });

  it('id has trialCalloutEnterprise key', () => {
    expect(idJson.download?.trialCalloutEnterprise).toBeTruthy();
  });

  it('en and id trialCallout texts differ (translated)', () => {
    expect(enJson.download.trialCallout).not.toBe(idJson.download.trialCallout);
  });

  it('en and id trialCalloutEnterprise texts differ (translated)', () => {
    expect(enJson.download.trialCalloutEnterprise).not.toBe(idJson.download.trialCalloutEnterprise);
  });
});
