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
const originalWarn = console.warn;

console.error = (...args: unknown[]) => {
  const arg0 = args[0];
  const msg =
    typeof arg0 === 'string'
      ? arg0
      : arg0 && typeof arg0 === 'object' && 'message' in arg0
        ? String((arg0 as { message?: unknown }).message)
        : String(arg0 ?? '');
  if (msg.includes('[@fluent/react]') && msg.includes('did not match any messages')) {
    return;
  }
  if (msg.includes('was not wrapped in act') || msg.includes('flushSync was called from inside')) {
    return;
  }
  if (msg.includes('validateDOMNesting') || msg.includes('punycode module is deprecated')) {
    return;
  }
  originalError(...args);
};

console.warn = (...args: unknown[]) => {
  const arg0 = args[0];
  const msg =
    typeof arg0 === 'string'
      ? arg0
      : arg0 && typeof arg0 === 'object' && 'message' in arg0
        ? String((arg0 as { message?: unknown }).message)
        : String(arg0 ?? '');
  if (msg.includes('[@fluent/react]') && msg.includes('did not match any messages')) {
    return;
  }
  if (msg.includes('was not wrapped in act') || msg.includes('flushSync was called from inside')) {
    return;
  }
  if (msg.includes('validateDOMNesting') || msg.includes('punycode module is deprecated')) {
    return;
  }
  originalWarn(...args);
};



