// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { createRoot } from 'react-dom/client';
import { act } from 'react';
import SearchModal from '../SearchModal';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

// ── Helpers ──────────────────────────────────────────────────────────

async function renderModal(isOpen = true, onClose = vi.fn(), locale = 'en') {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);

  await act(async () => {
    root.render(<SearchModal isOpen={isOpen} onClose={onClose} locale={locale} />);
  });

  return {
    container,
    root,
    unmount: async () => {
      await act(async () => {
        root.unmount();
      });
      container.remove();
    },
  };
}

async function renderOpen(locale = 'en') {
  const m = await renderModal(true, vi.fn(), locale);
  // Wait for the mounted state + focus timeout to settle
  await act(async () => {
    await new Promise((r) => setTimeout(r, 10));
  });
  const options = () => Array.from(document.body.querySelectorAll('a[role="option"]')) as HTMLElement[];
  const selected = () => options().findIndex((o) => o.getAttribute('aria-selected') === 'true');
  return {
    ...m,
    options,
    selected,
    press: async (key: string, opts: KeyboardEventInit = {}) => {
      await act(async () => {
        window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...opts }));
      });
    },
  };
}

function setInputValue(input: HTMLInputElement, value: string) {
  const nativeSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
  nativeSetter.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

beforeEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = '';
});

afterEach(() => {
  document.body.innerHTML = '';
});

// ── Basic rendering (existing) ───────────────────────────────────────

describe('SearchModal Component', () => {
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
      setInputValue(input, 'pricing');
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
      setInputValue(input, 'xyznonexistentterm123');
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

// ── Keyboard navigation ──────────────────────────────────────────────

describe('SearchModal — keyboard navigation', () => {
  it('starts with the first option selected', async () => {
    const m = await renderOpen();
    try {
      expect(m.selected()).toBe(0);
    } finally {
      await m.unmount();
    }
  });

  it('ArrowDown moves selection forward', async () => {
    const m = await renderOpen();
    try {
      await m.press('ArrowDown');
      expect(m.selected()).toBe(1);
    } finally {
      await m.unmount();
    }
  });

  it('ArrowDown wraps from the last option to the first', async () => {
    const m = await renderOpen();
    try {
      for (let i = 0; i < 8; i++) await m.press('ArrowDown');
      expect(m.selected()).toBe(0);
    } finally {
      await m.unmount();
    }
  });

  it('ArrowUp wraps from the first option to the last', async () => {
    const m = await renderOpen();
    try {
      await m.press('ArrowUp');
      expect(m.selected()).toBe(m.options().length - 1);
    } finally {
      await m.unmount();
    }
  });

  it('resets selection to 0 when the query changes', async () => {
    const m = await renderOpen();
    try {
      await m.press('ArrowDown');
      expect(m.selected()).toBe(1);

      const input = document.body.querySelector('input[type="search"]') as HTMLInputElement;
      await act(async () => {
        setInputValue(input, 'qris');
      });

      expect(m.selected()).toBe(0);
    } finally {
      await m.unmount();
    }
  });

  it('highlights the hovered option', async () => {
    const m = await renderOpen();
    try {
      const third = m.options()[2];
      // React onMouseEnter is triggered by native mouseover with relatedTarget
      // outside the target element.
      await act(async () => {
        third.dispatchEvent(
          new MouseEvent('mouseover', { bubbles: true, relatedTarget: document.body }),
        );
      });
      expect(m.selected()).toBe(2);
    } finally {
      await m.unmount();
    }
  });

  it('filters by keywords (docs)', async () => {
    const m = await renderOpen();
    try {
      const input = document.body.querySelector('input[type="search"]') as HTMLInputElement;
      await act(async () => {
        setInputValue(input, 'cloud-sync');
      });
      const opts = m.options();
      expect(opts.length).toBe(1);
      expect(opts[0].textContent?.toLowerCase()).toContain('cloud sync');
    } finally {
      await m.unmount();
    }
  });

  it('uses localized titles for the id locale', async () => {
    const m = await renderOpen('id');
    try {
      expect(m.options().some((o) => o.textContent?.includes('Beranda'))).toBe(true);
    } finally {
      await m.unmount();
    }
  });
});

// ── Search index completeness ────────────────────────────────────────

describe('SearchModal — search index completeness', () => {
  /**
   * Every page in src/pages/[locale]/ must be reachable via Ctrl+K search.
   * This test reads the actual page files and compares them against the
   * hardcoded search index in SearchModal.tsx. If someone adds a page but
   * forgets the search entry, this test fails.
   */
  it('search index includes every page from src/pages/[locale]/', () => {
    const pagesDir = join(process.cwd(), 'src', 'pages', '[locale]');
    const pageFiles = readFileSync(
      join(process.cwd(), 'src', 'components', 'SearchModal.tsx'),
      'utf8',
    );

    // Extract all id: '...' values from the searchItems array
    const indexIds = [...pageFiles.matchAll(/id:\s*['"]([^'"]+)['"]/g)].map((m) => m[1]);

    // Map page filenames to expected search index slugs
    const pageSlugs = readdirSync(pagesDir, 'utf8')
      .filter((f) => f.endsWith('.astro'))
      .map((f) => f.replace('.astro', ''))
      .filter((f) => !f.startsWith('[')) // skip [...slug].astro dynamic routes
      .map((f) => {
        if (f === 'index') return 'home';
        if (f.startsWith('untuk-')) return f.replace('untuk-', '');
        return f;
      });

    const missing = pageSlugs.filter((slug) => !indexIds.includes(slug));

    expect(
      missing,
      `Pages missing from search index: ${missing.join(', ')}. ` +
        `Add them to SearchModal.tsx searchItems array with id, title, category, url, and keywords.`,
    ).toEqual([]);
  });

  it('every search index page entry has a title in both en and id', () => {
    const source = readFileSync(
      join(process.cwd(), 'src', 'components', 'SearchModal.tsx'),
      'utf8',
    );

    // Find all title expressions: locale === 'id' ? '...' : '...'
    const titlePairs = [
      ...source.matchAll(/title:\s*locale\s*===\s*['"]id['"]\s*\?\s*['"]([^'"]+)['"]\s*:\s*['"]([^'"]+)['"]/g),
    ];

    expect(titlePairs.length).toBeGreaterThan(0);

    for (const [, idTitle, enTitle] of titlePairs) {
      expect(enTitle.length, `EN title for must not be empty`).toBeGreaterThan(0);
      expect(idTitle.length, `ID title must not be empty`).toBeGreaterThan(0);
    }
  });
});
