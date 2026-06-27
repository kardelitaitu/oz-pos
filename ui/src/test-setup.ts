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
