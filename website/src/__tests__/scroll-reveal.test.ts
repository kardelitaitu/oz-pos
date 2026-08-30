// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for Base.astro client-side scroll-reveal script.
 *
 * The script uses IntersectionObserver to add a `.revealed` class to
 * `[data-reveal]` elements when they enter the viewport. This drives
 * fade-in animations across every page.
 */

const BASE_SRC = readFileSync(
  join(import.meta.dirname, '..', 'layouts', 'Base.astro'),
  'utf-8',
);

function stripTypeScript(code: string): string {
  return code
    .replace(/<\w+>/g, '')
    .replace(/\bas\s+\w+/g, '');
}

function extractScript(): string {
  const match = BASE_SRC.match(/<script>([\s\S]*?)<\/script>/);
  if (!match) throw new Error('Could not extract <script> from Base.astro');
  return stripTypeScript(match[1].trim());
}

/**
 * Run the script via eval() — needed because IntersectionObserver is a
 * custom global that <script> injection doesn't share in vitest's jsdom.
 * eval() runs in the test's context, sharing both document and globals.
 */
function runScript(code: string): void {
  eval(code);
}

// ─── Mock IntersectionObserver ───────────────────────────────────────

interface MockObserver {
  callback: IntersectionObserverCallback;
  options: IntersectionObserverInit;
  observed: Set<Element>;
}

let observerInstances: MockObserver[] = [];

function installMockObserver(): void {
  observerInstances = [];

  (window as any).IntersectionObserver = class {
    callback: IntersectionObserverCallback;
    options: IntersectionObserverInit;
    observed = new Set<Element>();

    constructor(callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
      this.callback = callback;
      this.options = options ?? {};
      observerInstances.push({
        callback: this.callback,
        options: this.options,
        observed: this.observed,
      });
    }

    observe(target: Element) { this.observed.add(target); }
    unobserve(target: Element) { this.observed.delete(target); }
    disconnect() { this.observed.clear(); }
    takeRecords() { return []; }
  };
}

function triggerObserver(instance: MockObserver, entries: Partial<IntersectionObserverEntry>[]): void {
  const fullEntries = entries.map((e) => ({
    target: document.body,
    isIntersecting: false,
    intersectionRatio: 0,
    boundingClientRect: {} as DOMRectReadOnly,
    intersectionRect: {} as DOMRectReadOnly,
    rootBounds: null,
    time: 0,
    ...e,
  })) as IntersectionObserverEntry[];
  instance.callback(fullEntries, instance as unknown as IntersectionObserver);
}

// ─── Source structure tests ──────────────────────────────────────────

describe('Base.astro scroll-reveal source', () => {
  it('has initScrollReveal function', () => {
    expect(BASE_SRC).toContain('initScrollReveal');
  });

  it('queries data-reveal elements', () => {
    expect(BASE_SRC).toContain('[data-reveal]');
  });

  it('excludes already-revealed elements from initial query', () => {
    expect(BASE_SRC).toContain('[data-reveal]:not(.revealed)');
  });

  it('creates IntersectionObserver', () => {
    expect(BASE_SRC).toContain('new IntersectionObserver');
  });

  it('adds revealed class on intersection', () => {
    expect(BASE_SRC).toContain("classList.add('revealed')");
  });

  it('unobserves after reveal', () => {
    expect(BASE_SRC).toContain('observer.unobserve');
  });

  it('uses threshold 0.05', () => {
    expect(BASE_SRC).toContain('threshold: 0.05');
  });

  it('uses rootMargin for negative bottom offset', () => {
    expect(BASE_SRC).toContain('rootMargin');
  });

  it('checks if element is already in viewport on load', () => {
    expect(BASE_SRC).toContain('getBoundingClientRect');
  });

  it('adds revealed class with delay for in-view elements', () => {
    expect(BASE_SRC).toContain('setTimeout');
  });

  it('listens for DOMContentLoaded', () => {
    expect(BASE_SRC).toContain('DOMContentLoaded');
  });

  it('listens for astro:page-load', () => {
    expect(BASE_SRC).toContain('astro:page-load');
  });
});

// ─── Scroll-reveal behavior tests ────────────────────────────────────

describe('Scroll-reveal behavior', () => {
  const SCRIPT = extractScript();
  let savedInnerHeight: number;

  beforeEach(() => {
    document.body.innerHTML = '';
    savedInnerHeight = window.innerHeight;
    (window as any).innerHeight = 768;
    installMockObserver();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    (window as any).innerHeight = savedInnerHeight;
    document.body.innerHTML = '';
    observerInstances = [];
  });

  it('adds .revealed to elements already in viewport on load', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 100, bottom: 300, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 100,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    // Extract and run only the initScrollReveal function + call it
    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);
    vi.advanceTimersByTime(60);

    expect(el.classList.contains('revealed')).toBe(true);
  });

  it('does NOT add .revealed immediately (waits for delay)', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 100, bottom: 300, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 100,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(el.classList.contains('revealed')).toBe(false);
  });

  it('observes elements not in viewport', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances.length).toBe(1);
    expect(observerInstances[0].observed.has(el)).toBe(true);
  });

  it('adds .revealed when observer fires with isIntersecting', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    triggerObserver(observerInstances[0], [{ target: el, isIntersecting: true }]);

    expect(el.classList.contains('revealed')).toBe(true);
  });

  it('unobserves element after reveal', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances[0].observed.has(el)).toBe(true);

    triggerObserver(observerInstances[0], [{ target: el, isIntersecting: true }]);

    expect(observerInstances[0].observed.has(el)).toBe(false);
  });

  it('does NOT add .revealed when observer fires without isIntersecting', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    triggerObserver(observerInstances[0], [{ target: el, isIntersecting: false }]);

    expect(el.classList.contains('revealed')).toBe(false);
    expect(observerInstances[0].observed.has(el)).toBe(true);
  });

  it('handles multiple data-reveal elements', () => {
    const el1 = document.createElement('div');
    el1.setAttribute('data-reveal', '');
    el1.getBoundingClientRect = () => ({
      top: 100, bottom: 300, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 100,
      toJSON: () => {},
    });
    const el2 = document.createElement('div');
    el2.setAttribute('data-reveal', '');
    el2.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el1);
    document.body.appendChild(el2);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    vi.advanceTimersByTime(60);
    expect(el1.classList.contains('revealed')).toBe(true);

    expect(observerInstances[0].observed.has(el2)).toBe(true);

    triggerObserver(observerInstances[0], [{ target: el2, isIntersecting: true }]);
    expect(el2.classList.contains('revealed')).toBe(true);
  });

  it('ignores elements without data-reveal attribute', () => {
    const el = document.createElement('div');
    el.textContent = 'No reveal';
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    vi.advanceTimersByTime(60);
    expect(el.classList.contains('revealed')).toBe(false);
    expect(observerInstances.length).toBe(0);
  });

  it('ignores elements that already have .revealed class', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.classList.add('revealed');
    el.getBoundingClientRect = () => ({
      top: 100, bottom: 300, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 100,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances.length).toBe(0);
  });

  it('does nothing when no data-reveal elements exist', () => {
    document.body.appendChild(document.createElement('p'));

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances.length).toBe(0);
  });

  it('uses threshold 0.05 in observer options', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances[0].options.threshold).toBe(0.05);
  });

  it('uses rootMargin with negative bottom offset', () => {
    const el = document.createElement('div');
    el.setAttribute('data-reveal', '');
    el.getBoundingClientRect = () => ({
      top: 5000, bottom: 5200, left: 0, right: 200,
      width: 200, height: 200, x: 0, y: 5000,
      toJSON: () => {},
    });
    document.body.appendChild(el);

    runScript(`
      ${SCRIPT}
      initScrollReveal();
    `);

    expect(observerInstances[0].options.rootMargin).toContain('-20px');
  });
});
