// Vitest global setup: load jest-dom matchers and any browser API
// polyfills the components need.

import '@testing-library/jest-dom/vitest';
import { beforeEach, vi } from 'vitest';

// ── Global mock: @tauri-apps/api/event ─────────────────────────
// SettingsContext uses a dynamic import('@tauri-apps/api/event')
// which per-file vi.mock() cannot intercept.  This global mock
// ensures the dynamic import resolves to a stub rather than the
// real Tauri module (which calls transformCallback, undefined in
// jsdom, and throws "Cannot read properties of undefined").
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// ── Global beforeEach: clean mocks and localStorage ─────────────
// Every test starts with a clean slate. Individual test files that
// need additional setup (e.g. mockImplementation overrides) add
// their own beforeEach after this global one runs.
beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
});

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

// ResizeObserver is not implemented/simulated in jsdom; stub it so virtualised grids render.
if (typeof window !== 'undefined') {
  window.ResizeObserver = class MockResizeObserver {
    callback: ResizeObserverCallback;
    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }
    observe(element: HTMLElement) {
      this.callback(
        [
          {
            target: element,
            contentRect: {
              width: 1024,
              height: 768,
              top: 0,
              left: 0,
              bottom: 768,
              right: 1024,
              x: 0,
              y: 0,
              toJSON: () => {},
            },
            borderBoxSize: [],
            contentBoxSize: [],
            devicePixelContentBoxSize: [],
          },
        ],
        this
      );
    }
    unobserve() {}
    disconnect() {}
  };
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

// ── Canvas mock ───────────────────────────────────────────────
// jsdom does not implement HTMLCanvasElement.prototype.getContext.
// Stub it so components that use canvas charts (CanvasPieChart,
// CanvasHeatmap, CanvasLineChart, NodeTopologyEditor) don't throw.
if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = (() => {
    // Return a minimal mock that chart/canvas code can call without
    // crashing. Individual tests that need real canvas behaviour
    // should override this mock.
    return null;
  }) as typeof HTMLCanvasElement.prototype.getContext;
}
