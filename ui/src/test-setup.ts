// Vitest global setup: load jest-dom matchers and any browser API
// polyfills the components need.

import '@testing-library/jest-dom/vitest';

// matchMedia is not implemented in jsdom; Fluent uses it for
// responsive layouts. Stub it.
if (typeof window !== 'undefined' && !window.matchMedia) {
  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}

// ── Suppress @fluent/react missing-key console errors ────────────
//
// Components may render <Localized id="…"> with dynamic action keys
// (e.g., audit log action labels, status badges) that aren't in
// every test's FluentBundle. The real app provides all keys via the
// joined .ftl bundles, but individual tests load only the subset
// they need.  Suppressing the console.error avoids noisy test
// output while still allowing tests that need specific translations
// to set up their own bundles via <LocalizationProvider>.
const originalError = console.error;
console.error = (...args: unknown[]) => {
  const msg = typeof args[0] === 'string' ? args[0] : '';
  // Only suppress @fluent/react missing-key warnings — not all
  // @fluent/react errors, which could mask genuine issues.
  if (msg.includes('[@fluent/react]') && msg.includes('did not match any messages')) {
    return;
  }
  originalError(...args);
};
