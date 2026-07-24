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

// ── Global mock: @/contexts/WorkspaceContext ──────────────────────
// Many component tests render screens that call `useWorkspace()`
// or `useWorkspaceScope()` without wrapping the tree in a real
// `<WorkspaceProvider>`. To avoid wrapping every test we keep the
// real exports (`WorkspaceProvider`, `WorkspaceContext`,
// `WorkspaceScopeContext`, `WorkspaceScope`, types) and override
// only the hooks to return a safe no-op default.
//
// Rationale for always returning the safe default from the hooks:
//   * Any value passed via `renderWithWorkspace({ workspace })` or
//     an explicit `<WorkspaceContext.Provider value=...>` is
//     ignored by hooks in this fallback. Tests that need custom
//     workspace values can override per-test via
//     `vi.mocked(useWorkspace).mockReturnValue(...)`.
//   * This keeps the mock robust against problems detected in the
//     React.useContext path (e.g. `React.useContext is not a
//     function` when `importOriginal('react')` returns a proxy
//     object without the real hooks).
//
// Tests that intentionally exercise the real provider
// (currently `WorkspaceContext.test.tsx`) opt out with
// `vi.unmock('@/contexts/WorkspaceContext')` at the top.
vi.mock('@/contexts/WorkspaceContext', async (importOriginal) => {
  const actual = await importOriginal<
    typeof import('@/contexts/WorkspaceContext')
  >();

  const safeWorkspaceDefault = {
    activeWorkspace: null,
    setActiveWorkspace: vi.fn(),
    activeInstance: null,
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
    sessionToken: 'mock-session-token',
    swapSessionToken: vi.fn(() => Promise.resolve()),
  };

  // Non-null scope keeps components like KioskScreen and KdsScreen
  // (which dereference .storeId/.instanceId/.typeKey) safe under the
  // global stub — null would crash with a different NPE than the
  // provider error we are replacing. typeKey is intentionally
  // generic ('default') so components that branch on typeKey
  // (e.g. KdsScreen vs. RestaurantMenu routing) do not silently
  // render the wrong variant. Tests that need a specific typeKey
  // should use vi.mocked(useWorkspaceScope).mockReturnValue(...).
  const safeScopeDefault = {
    storeId: 'default',
    instanceId: 'default',
    typeKey: 'default',
  };

  return {
    ...actual,
    useWorkspace: vi.fn().mockImplementation(() => safeWorkspaceDefault),
    useWorkspaceScope: vi.fn().mockImplementation(() => safeScopeDefault),
  };
});

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
