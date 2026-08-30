// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for DocSidebar.astro client-side search filter.
 *
 * The script filters doc items by title match when the user types in the
 * search input. It hides/shows groups and toggles a "no results" message.
 * Each sidebar instance is scoped independently (mobile + desktop).
 */

const SIDEBAR_SRC = readFileSync(
  join(import.meta.dirname, '..', 'DocSidebar.astro'),
  'utf-8',
);

/** Strip TypeScript type annotations so the script can run in plain JS. */
function stripTypeScript(code: string): string {
  // Remove generic type parameters: <HTMLElement>, <HTMLInputElement>, etc.
  return code.replace(/<\w+>/g, '');
}

function extractScript(): string {
  const match = SIDEBAR_SRC.match(/<script>([\s\S]*?)<\/script>/);
  if (!match) throw new Error('Could not extract <script> from DocSidebar.astro');
  return stripTypeScript(match[1].trim());
}

/**
 * Inject the extracted script into the document via a <script> element.
 * This runs in the same realm as the test — shared document, Event, etc.
 */
function injectScript(code: string): void {
  const el = document.createElement('script');
  el.textContent = code;
  document.body.appendChild(el);
}

function buildSidebarDOM(): HTMLElement {
  const sidebar = document.createElement('div');
  sidebar.className = 'doc-sidebar';

  const input = document.createElement('input');
  input.type = 'search';
  input.setAttribute('data-doc-search', '');
  sidebar.appendChild(input);

  const none = document.createElement('p');
  none.setAttribute('data-doc-none', '');
  none.classList.add('hidden');
  none.textContent = 'No results';
  sidebar.appendChild(none);

  const g1 = document.createElement('div');
  g1.setAttribute('data-doc-group', '');
  const g1Title = document.createElement('p');
  g1Title.textContent = 'Getting Started';
  g1.appendChild(g1Title);
  const ul1 = document.createElement('ul');
  for (const title of ['Installation', 'Quick Start']) {
    const li = document.createElement('li');
    li.setAttribute('data-doc-item', '');
    const a = document.createElement('a');
    a.textContent = title;
    a.href = `/docs/${title.toLowerCase().replace(/\s+/g, '-')}`;
    li.appendChild(a);
    ul1.appendChild(li);
  }
  g1.appendChild(ul1);
  sidebar.appendChild(g1);

  const g2 = document.createElement('div');
  g2.setAttribute('data-doc-group', '');
  const g2Title = document.createElement('p');
  g2Title.textContent = 'Configuration';
  g2.appendChild(g2Title);
  const ul2 = document.createElement('ul');
  for (const title of ['Database Setup', 'Environment Variables']) {
    const li = document.createElement('li');
    li.setAttribute('data-doc-item', '');
    const a = document.createElement('a');
    a.textContent = title;
    a.href = `/docs/${title.toLowerCase().replace(/\s+/g, '-')}`;
    li.appendChild(a);
    ul2.appendChild(li);
  }
  g2.appendChild(ul2);
  sidebar.appendChild(g2);

  return sidebar;
}

// ─── Source structure tests ──────────────────────────────────────────

describe('DocSidebar source structure', () => {
  it('has data-doc-search input', () => {
    expect(SIDEBAR_SRC).toContain('data-doc-search');
  });

  it('has data-doc-none element for no-results message', () => {
    expect(SIDEBAR_SRC).toContain('data-doc-none');
  });

  it('has data-doc-group elements for category groups', () => {
    expect(SIDEBAR_SRC).toContain('data-doc-group');
  });

  it('has data-doc-item elements for individual docs', () => {
    expect(SIDEBAR_SRC).toContain('data-doc-item');
  });

  it('has a client script with input event listener', () => {
    expect(SIDEBAR_SRC).toContain("addEventListener('input'");
  });

  it('scopes filter to each sidebar instance independently', () => {
    expect(SIDEBAR_SRC).toContain('querySelectorAll');
    expect(SIDEBAR_SRC).toContain('.doc-sidebar');
  });

  it('hides items with display: none on non-match', () => {
    expect(SIDEBAR_SRC).toContain("style.display = match ? '' : 'none'");
  });

  it('toggles hidden class on no-results message', () => {
    expect(SIDEBAR_SRC).toContain("none.classList.toggle('hidden', anyVisible)");
  });
});

// ─── Filter behavior tests ───────────────────────────────────────────

describe('DocSidebar search filter behavior', () => {
  const SCRIPT = extractScript();
  let sidebar: HTMLElement;
  let input: HTMLInputElement;

  beforeEach(() => {
    document.body.innerHTML = '';
    sidebar = buildSidebarDOM();
    document.body.appendChild(sidebar);
    input = sidebar.querySelector('[data-doc-search]') as HTMLInputElement;
    injectScript(SCRIPT);
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('shows all items when input is empty', () => {
    const items = sidebar.querySelectorAll('[data-doc-item]');
    for (const item of items) {
      expect((item as HTMLElement).style.display).toBe('');
    }
  });

  it('shows all groups when input is empty', () => {
    const groups = sidebar.querySelectorAll('[data-doc-group]');
    for (const group of groups) {
      expect((group as HTMLElement).style.display).toBe('');
    }
  });

  it('hides no-results message when input is empty', () => {
    const none = sidebar.querySelector('[data-doc-none]');
    expect(none?.classList.contains('hidden')).toBe(true);
  });

  it('filters items by title match (case-insensitive)', () => {
    input.value = 'install';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const items = sidebar.querySelectorAll('[data-doc-item]');
    const visible = [...items].filter((el) => (el as HTMLElement).style.display !== 'none');
    const hidden = [...items].filter((el) => (el as HTMLElement).style.display === 'none');

    expect(visible).toHaveLength(1);
    expect(visible[0].textContent?.trim()).toBe('Installation');
    expect(hidden).toHaveLength(3);
  });

  it('filters items with partial match', () => {
    input.value = 'setup';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const items = sidebar.querySelectorAll('[data-doc-item]');
    const visible = [...items].filter((el) => (el as HTMLElement).style.display !== 'none');

    expect(visible).toHaveLength(1);
    expect(visible[0].textContent?.trim()).toBe('Database Setup');
  });

  it('hides groups with no matching items', () => {
    input.value = 'database';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const groups = sidebar.querySelectorAll('[data-doc-group]');
    const g1 = groups[0];
    const g2 = groups[1];

    expect((g1 as HTMLElement).style.display).toBe('none');
    expect((g2 as HTMLElement).style.display).toBe('');
  });

  it('shows no-results message when no items match', () => {
    input.value = 'xyznonexistent';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const none = sidebar.querySelector('[data-doc-none]');
    expect(none?.classList.contains('hidden')).toBe(false);
  });

  it('hides no-results message when at least one item matches', () => {
    input.value = 'xyznonexistent';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(sidebar.querySelector('[data-doc-none]')?.classList.contains('hidden')).toBe(false);

    input.value = 'quick';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    expect(sidebar.querySelector('[data-doc-none]')?.classList.contains('hidden')).toBe(true);
  });

  it('trims whitespace from query', () => {
    input.value = '  install  ';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const items = sidebar.querySelectorAll('[data-doc-item]');
    const visible = [...items].filter((el) => (el as HTMLElement).style.display !== 'none');
    expect(visible).toHaveLength(1);
    expect(visible[0].textContent?.trim()).toBe('Installation');
  });

  it('is case-insensitive', () => {
    input.value = 'INSTALLATION';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const items = sidebar.querySelectorAll('[data-doc-item]');
    const visible = [...items].filter((el) => (el as HTMLElement).style.display !== 'none');
    expect(visible).toHaveLength(1);
  });

  it('shows all items again after clearing the search', () => {
    input.value = 'install';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    input.value = '';
    input.dispatchEvent(new Event('input', { bubbles: true }));

    const items = sidebar.querySelectorAll('[data-doc-item]');
    const visible = [...items].filter((el) => (el as HTMLElement).style.display !== 'none');
    expect(visible).toHaveLength(4);

    const groups = sidebar.querySelectorAll('[data-doc-group]');
    for (const group of groups) {
      expect((group as HTMLElement).style.display).toBe('');
    }
  });
});

// ─── Scoped instance tests ───────────────────────────────────────────

describe('DocSidebar scoped instances', () => {
  const SCRIPT = extractScript();

  beforeEach(() => {
    document.body.innerHTML = '';
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('filters each sidebar independently', () => {
    const s1 = buildSidebarDOM();
    const s2 = buildSidebarDOM();
    document.body.appendChild(s1);
    document.body.appendChild(s2);
    injectScript(SCRIPT);

    const input1 = s1.querySelector('[data-doc-search]') as HTMLInputElement;
    input1.value = 'install';
    input1.dispatchEvent(new Event('input', { bubbles: true }));

    const s1Items = s1.querySelectorAll('[data-doc-item]');
    const s1Visible = [...s1Items].filter((el) => (el as HTMLElement).style.display !== 'none');
    expect(s1Visible).toHaveLength(1);

    const s2Items = s2.querySelectorAll('[data-doc-item]');
    const s2Visible = [...s2Items].filter((el) => (el as HTMLElement).style.display !== 'none');
    expect(s2Visible).toHaveLength(4);
  });

  it('handles missing search input gracefully', () => {
    const sidebar = document.createElement('div');
    sidebar.className = 'doc-sidebar';
    const none = document.createElement('p');
    none.setAttribute('data-doc-none', '');
    none.classList.add('hidden');
    sidebar.appendChild(none);
    document.body.appendChild(sidebar);

    expect(() => injectScript(SCRIPT)).not.toThrow();
  });
});

// ─── i18n key tests ──────────────────────────────────────────────────

describe('DocSidebar i18n keys', () => {
  const enJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'en.json'), 'utf-8'),
  );
  const idJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'id.json'), 'utf-8'),
  );

  it('en has docsSearch.placeholder', () => {
    expect(enJson.docsSearch?.placeholder).toBeTruthy();
  });

  it('en has docsSearch.noResults', () => {
    expect(enJson.docsSearch?.noResults).toBeTruthy();
  });

  it('id has docsSearch.placeholder', () => {
    expect(idJson.docsSearch?.placeholder).toBeTruthy();
  });

  it('id has docsSearch.noResults', () => {
    expect(idJson.docsSearch?.noResults).toBeTruthy();
  });

  it('en and id placeholder texts differ (translated)', () => {
    expect(enJson.docsSearch.placeholder).not.toBe(idJson.docsSearch.placeholder);
  });

  it('en and id noResults texts differ (translated)', () => {
    expect(enJson.docsSearch.noResults).not.toBe(idJson.docsSearch.noResults);
  });
});
