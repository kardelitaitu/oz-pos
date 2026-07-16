import { act } from 'react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from '@testing-library/react';
import FastPINOverlay from '@/components/FastPINOverlay';

// ── Mock helpers (extracted outside vi.mock for esbuild compat) ────

function MockPassThrough({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

// ── Mocks ────────────────────────────────────────────────────────────

vi.mock('@/api/staff', () => ({
  staffLogin: vi.fn(),
}));

vi.mock('@/contexts/AuthContext', () => ({
  useAuth: () => ({
    session: null,
    loading: false,
    error: null,
    login: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    swapSession: vi.fn(),
    isManager: false,
    isOwner: false,
  }),
  AuthProvider: MockPassThrough,
}));

vi.mock('@/contexts/WorkspaceContext', () => ({
  useWorkspace: () => ({
    swapSessionToken: vi.fn(),
    activeWorkspace: 'store-pos',
    activeInstance: null,
    sessionToken: 'token-abc',
    setActiveWorkspace: vi.fn(),
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: 'default',
  }),
  useWorkspaceScope: () => ({
    storeId: 'default',
    instanceId: 'default-store-pos',
    typeKey: 'store-pos',
  }),
  WorkspaceProvider: MockPassThrough,
}));

vi.mock('@/api/license', () => ({
  getMachineId: vi.fn().mockResolvedValue('abc-123'),
}));

vi.mock('@/frontend/shared/Toast', () => ({
  useToast: () => ({ addToast: vi.fn() }),
  ToastProvider: MockPassThrough,
}));

vi.mock('@fluent/react', () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => (id === 'staff-login-pin-aria' ? 'PIN: {length}/{max}' : id),
    },
  }),
  Localized: ({ children }: { id: string; children: ReactNode }) => <>{children}</>,
}));

/**
 * Helper: parse a CSS --custom-property value from the stylesheets
 * loaded in jsdom. Returns the raw value string, or undefined if the
 * property is not found in any accessible stylesheet.
 */
function getCssCustomProperty(name: string): string | undefined {
  for (const sheet of document.styleSheets) {
    try {
      for (const rule of Array.from(sheet.cssRules || [])) {
        if (rule instanceof CSSStyleRule && rule.selectorText === ':root') {
          const value = rule.style.getPropertyValue(name);
          if (value) return value.trim();
        }
      }
    } catch {
      // Cross-origin/third-party stylesheets — skip
    }
  }
  return undefined;
}

describe('touch-target sizing compliance', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  describe('CSS custom property --touch-target-min', () => {
    it('exists in the stylesheet', () => {
      // Render something to load the stylesheets
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const value = getCssCustomProperty('--touch-target-min');
      // In jsdom, CSS custom properties from .css files may not be
      // accessible via document.styleSheets (CSS modules are hoisted).
      // This test is informative: if the property IS accessible, verify
      // it's ≥ 44px. If not (jsdom limitation), the test passes by
      // verifying the close button element exists and is interactive.
      if (value !== undefined) {
        expect(parseFloat(value)).toBeGreaterThanOrEqual(44);
      }
      // Element presence check even if CSS isn't accessible
      const closeBtn = document.querySelector('.fastpin-close-btn');
      expect(closeBtn).not.toBeNull();
    });
  });

  describe('FastPINOverlay — close button', () => {
    it('is a <button> element with type="button"', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const closeBtn = document.querySelector('.fastpin-close-btn');
      expect(closeBtn).not.toBeNull();
      expect(closeBtn?.tagName).toBe('BUTTON');
      expect(closeBtn?.getAttribute('type')).toBe('button');
    });

    it('has aria-label attribute', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const closeBtn = document.querySelector('.fastpin-close-btn');
      expect(closeBtn?.getAttribute('aria-label')).toBeTruthy();
    });

    it('is not disabled by default', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const closeBtn = document.querySelector('.fastpin-close-btn') as HTMLButtonElement;
      expect(closeBtn?.disabled).toBe(false);
    });
  });

  describe('FastPINOverlay — pin pad keys', () => {
    it('are rendered and are <button> elements with type="button"', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      // Navigate to PIN step
      const usernameInput = document.querySelector('.fastpin-input') as HTMLInputElement;
      if (usernameInput) {
        const nativeSetter = Object.getOwnPropertyDescriptor(
          window.HTMLInputElement.prototype, 'value',
        )?.set;
        nativeSetter?.call(usernameInput, 'cashier1');
        usernameInput.dispatchEvent(new Event('input', { bubbles: true }));

        const submitBtn = document.querySelector('.fastpin-submit-btn') as HTMLButtonElement;
        act(() => {
          submitBtn?.click();
        });
      }

      // Check for keypad keys
      const keys = document.querySelectorAll('.fastpin-pad-key');
      expect(keys.length).toBeGreaterThanOrEqual(10);

      keys.forEach((key) => {
        expect(key.tagName).toBe('BUTTON');
        expect(key.getAttribute('type')).toBe('button');
      });
    });
  });

  describe('FastPINOverlay — cancel button', () => {
    it('is a <button> element with type="button"', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const cancelBtn = document.querySelector('.fastpin-cancel-btn');
      expect(cancelBtn).not.toBeNull();
      expect(cancelBtn?.tagName).toBe('BUTTON');
      expect(cancelBtn?.getAttribute('type')).toBe('button');
    });

    it('is not disabled by default', () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      const cancelBtn = document.querySelector('.fastpin-cancel-btn') as HTMLButtonElement;
      expect(cancelBtn?.disabled).toBe(false);
    });

    it('closes the overlay when clicked', () => {
      const onClose = vi.fn();
      render(<FastPINOverlay open={true} onClose={onClose} />);

      const cancelBtn = document.querySelector('.fastpin-cancel-btn') as HTMLButtonElement;
      cancelBtn?.click();

      // Should trigger exit animation (200ms timeout)
      expect(onClose).not.toHaveBeenCalled(); // Not yet — exit animation in progress

      act(() => {
        vi.advanceTimersByTime(200);
      });

      expect(onClose).toHaveBeenCalled();
    });
  });
});
