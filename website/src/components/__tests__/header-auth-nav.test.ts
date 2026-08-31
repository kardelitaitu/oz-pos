// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for Header.astro client-side auth nav script.
 *
 * The `updateAuthNav` function reads `sessionStorage.oz_session` and swaps
 * login buttons between "Sign in" → /login and "Account" → /account.
 * If this breaks, logged-in users see "Sign in" and vice versa.
 */

const HEADER_SRC = readFileSync(
  join(import.meta.dirname, '..', 'Header.astro'),
  'utf-8',
);

/** Strip TypeScript annotations so the script can run in plain JS. */
function stripTypeScript(code: string): string {
  return code
    .replace(/<\w+>/g, '')           // <HTMLElement>, <HTMLAnchorElement>
    .replace(/\bas\s+\w+/g, '')      // as HTMLElement, as HTMLAnchorElement
    .replace(/\bas\s+\w+(\.\w+)*/g, ''); // as HTMLElement.Closest etc.
}

function extractScript(): string {
  const match = HEADER_SRC.match(/<script>([\s\S]*?)<\/script>/);
  if (!match) throw new Error('Could not extract <script> from Header.astro');
  return stripTypeScript(match[1].trim());
}

/** Inject the script via <script> element for proper realm sharing. */
function injectScript(code: string): void {
  const el = document.createElement('script');
  el.textContent = code;
  document.body.appendChild(el);
}

function buildHeaderDOM(): HTMLElement {
  const header = document.createElement('header');

  const desktopLink = document.createElement('a');
  desktopLink.setAttribute('data-auth-nav', '');
  desktopLink.setAttribute('data-locale', 'en');
  desktopLink.setAttribute('data-login-text', 'Sign in');
  desktopLink.setAttribute('data-account-text', 'Account');
  desktopLink.href = '/en/login';
  desktopLink.textContent = 'Sign in';
  header.appendChild(desktopLink);

  const mobileLink = document.createElement('a');
  mobileLink.setAttribute('data-auth-nav', '');
  mobileLink.setAttribute('data-locale', 'id');
  mobileLink.setAttribute('data-login-text', 'Masuk');
  mobileLink.setAttribute('data-account-text', 'Akun');
  mobileLink.href = '/id/login';
  mobileLink.textContent = 'Masuk';
  header.appendChild(mobileLink);

  return header;
}

// ─── Source structure tests ──────────────────────────────────────────

describe('Header source structure', () => {
  it('has updateAuthNav function', () => {
    expect(HEADER_SRC).toContain('updateAuthNav');
  });

  it('reads sessionStorage oz_session', () => {
    expect(HEADER_SRC).toContain("sessionStorage.getItem('oz_session')");
  });

  it('queries data-auth-nav elements', () => {
    expect(HEADER_SRC).toContain('[data-auth-nav]');
  });

  it('sets href to /account when authenticated', () => {
    expect(HEADER_SRC).toContain('/account');
  });

  it('sets href to /login when unauthenticated', () => {
    expect(HEADER_SRC).toContain('/login');
  });

  it('uses data-account-text attribute for authenticated label', () => {
    expect(HEADER_SRC).toContain('data-account-text');
  });

  it('uses data-login-text attribute for unauthenticated label', () => {
    expect(HEADER_SRC).toContain('data-login-text');
  });

  it('uses data-locale for locale detection', () => {
    expect(HEADER_SRC).toContain('data-locale');
  });

  it('registers astro:page-load event listener', () => {
    expect(HEADER_SRC).toContain("astro:page-load");
  });

  it('registers astro:after-swap event listener', () => {
    expect(HEADER_SRC).toContain("astro:after-swap");
  });

  it('registers storage event listener for cross-tab sync', () => {
    expect(HEADER_SRC).toContain("window.addEventListener('storage'");
  });

  it('calls updateAuthNav on initial load', () => {
    expect(HEADER_SRC).toContain('updateAuthNav();');
  });

  it('has mobile menu close-on-click handler', () => {
    expect(HEADER_SRC).toContain('details.removeAttribute');
  });
});

// ─── Auth nav behavior tests ─────────────────────────────────────────

describe('Header auth nav behavior', () => {
  const SCRIPT = extractScript();
  let header: HTMLElement;
  let desktopLink: HTMLAnchorElement;
  let mobileLink: HTMLAnchorElement;

  beforeEach(() => {
    document.body.innerHTML = '';
    sessionStorage.clear();
    header = buildHeaderDOM();
    document.body.appendChild(header);
    desktopLink = header.querySelectorAll('[data-auth-nav]')[0] as HTMLAnchorElement;
    mobileLink = header.querySelectorAll('[data-auth-nav]')[1] as HTMLAnchorElement;
    injectScript(SCRIPT);
  });

  afterEach(() => {
    sessionStorage.clear();
    document.body.innerHTML = '';
  });

  it('shows "Sign in" when no session exists', () => {
    expect(desktopLink.textContent).toBe('Sign in');
  });

  it('points to /login when no session exists', () => {
    expect(desktopLink.getAttribute('href')).toBe('/en/login');
  });

  it('shows localized login text for Indonesian locale', () => {
    expect(mobileLink.textContent).toBe('Masuk');
    expect(mobileLink.getAttribute('href')).toBe('/id/login');
  });

  it('shows "Account" when session exists', () => {
    sessionStorage.setItem('oz_session', 'some-session-id');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Account');
  });

  it('points to /account when session exists', () => {
    sessionStorage.setItem('oz_session', 'some-session-id');
    injectScript(SCRIPT);
    expect(desktopLink.getAttribute('href')).toBe('/en/account');
  });

  it('shows localized account text for Indonesian locale', () => {
    sessionStorage.setItem('oz_session', 'some-session-id');
    injectScript(SCRIPT);
    expect(mobileLink.textContent).toBe('Akun');
    expect(mobileLink.getAttribute('href')).toBe('/id/account');
  });

  it('switches from login to account when session is set', () => {
    expect(desktopLink.textContent).toBe('Sign in');
    sessionStorage.setItem('oz_session', 'new-session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Account');
    expect(desktopLink.getAttribute('href')).toBe('/en/account');
  });

  it('switches from account to login when session is cleared', () => {
    sessionStorage.setItem('oz_session', 'existing-session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Account');

    sessionStorage.removeItem('oz_session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Sign in');
    expect(desktopLink.getAttribute('href')).toBe('/en/login');
  });

  it('treats empty session string as unauthenticated', () => {
    sessionStorage.setItem('oz_session', '');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Sign in');
  });

  it('falls back to "Account" when data-account-text is missing', () => {
    desktopLink.removeAttribute('data-account-text');
    sessionStorage.setItem('oz_session', 'session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Account');
  });

  it('falls back to "Sign in" when data-login-text is missing', () => {
    desktopLink.removeAttribute('data-login-text');
    sessionStorage.removeItem('oz_session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Sign in');
  });

  it('falls back to "en" locale when data-locale is missing', () => {
    desktopLink.removeAttribute('data-locale');
    sessionStorage.setItem('oz_session', 'session');
    injectScript(SCRIPT);
    expect(desktopLink.getAttribute('href')).toBe('/en/account');
  });

  it('updates all auth-nav links at once', () => {
    sessionStorage.setItem('oz_session', 'session');
    injectScript(SCRIPT);
    expect(desktopLink.textContent).toBe('Account');
    expect(mobileLink.textContent).toBe('Akun');
  });

  it('closes mobile menu when a link inside details.group is clicked', () => {
    const details = document.createElement('details');
    details.className = 'group';
    details.setAttribute('open', '');
    const link = document.createElement('a');
    link.href = '/en/features';
    link.textContent = 'Features';
    details.appendChild(link);
    header.appendChild(details);

    injectScript(SCRIPT);

    link.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(details.hasAttribute('open')).toBe(false);
  });

  it('does not close details when clicking outside details.group', () => {
    const details = document.createElement('details');
    details.className = 'other';
    details.setAttribute('open', '');
    const link = document.createElement('a');
    link.href = '/en/features';
    details.appendChild(link);
    header.appendChild(details);

    injectScript(SCRIPT);

    link.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(details.hasAttribute('open')).toBe(true);
  });
});

// ─── i18n key tests ──────────────────────────────────────────────────

describe('Header i18n keys', () => {
  const enJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'en.json'), 'utf-8'),
  );
  const idJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'id.json'), 'utf-8'),
  );

  it('en has nav.login', () => {
    expect(enJson.nav?.login).toBeTruthy();
  });

  it('en has nav.account', () => {
    expect(enJson.nav?.account).toBeTruthy();
  });

  it('id has nav.login', () => {
    expect(idJson.nav?.login).toBeTruthy();
  });

  it('id has nav.account', () => {
    expect(idJson.nav?.account).toBeTruthy();
  });

  it('en and id login texts differ (translated)', () => {
    expect(enJson.nav.login).not.toBe(idJson.nav.login);
  });

  it('en and id account texts differ (translated)', () => {
    expect(enJson.nav.account).not.toBe(idJson.nav.account);
  });

  it('en has nav.home', () => {
    expect(enJson.nav?.home).toBeTruthy();
  });

  it('en has nav.features', () => {
    expect(enJson.nav?.features).toBeTruthy();
  });

  it('en has nav.pricing', () => {
    expect(enJson.nav?.pricing).toBeTruthy();
  });

  it('en has nav.download', () => {
    expect(enJson.nav?.download).toBeTruthy();
  });

  it('en has nav.support', () => {
    expect(enJson.nav?.support).toBeTruthy();
  });

  it('en has nav.docs', () => {
    expect(enJson.nav?.docs).toBeTruthy();
  });
});
