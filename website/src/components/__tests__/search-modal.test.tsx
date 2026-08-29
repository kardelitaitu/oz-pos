// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createRoot } from 'react-dom/client';
import { act } from 'react';
import SearchModal from '../SearchModal';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

describe('SearchModal Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = '';
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  async function renderModal(isOpen = true, onClose = vi.fn(), locale = 'en') {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<SearchModal isOpen={isOpen} onClose={onClose} locale={locale} />);
    });

    return {
      container,
      unmount: async () => {
        await act(async () => {
          root.unmount();
        });
        container.remove();
      },
    };
  }

  it('does not render when isOpen is false', async () => {
    const { unmount } = await renderModal(false);
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
    await unmount();
  });

  it('renders search input and initial results when open', async () => {
    const { unmount } = await renderModal(true);
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull();
    const input = document.body.querySelector('input[type="search"]') as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.placeholder).toContain('Search');
    await unmount();
  });

  it('filters results based on query', async () => {
    const { unmount } = await renderModal(true);
    const input = document.body.querySelector('input[type="search"]') as HTMLInputElement;

    await act(async () => {
      const nativeSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
      nativeSetter.call(input, 'pricing');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });

    const results = document.body.querySelectorAll('a[role="option"]');
    expect(results.length).toBeGreaterThan(0);
    const titles = Array.from(results).map((r) => r.textContent);
    expect(titles.some((t) => t?.toLowerCase().includes('pricing'))).toBe(true);
    await unmount();
  });

  it('shows no results message for unmatched query', async () => {
    const { unmount } = await renderModal(true);
    const input = document.body.querySelector('input[type="search"]') as HTMLInputElement;

    await act(async () => {
      const nativeSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
      nativeSetter.call(input, 'xyznonexistentterm123');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });

    expect(document.body.textContent).toContain('No matching results found');
    await unmount();
  });

  it('calls onClose when Escape key is pressed or backdrop is clicked', async () => {
    const onClose = vi.fn();
    const { unmount } = await renderModal(true, onClose);

    const backdrop = document.body.querySelector('[data-backdrop="true"]') as HTMLElement;
    if (backdrop) {
      await act(async () => {
        backdrop.click();
      });
      expect(onClose).toHaveBeenCalled();
    }

    await act(async () => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    expect(onClose).toHaveBeenCalled();

    await unmount();
  });

  it('calls onClose on mousedown outside the dialog', async () => {
    const onClose = vi.fn();
    const { unmount } = await renderModal(true, onClose);

    await act(async () => {
      document.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    expect(onClose).toHaveBeenCalled();

    await unmount();
  });
});
