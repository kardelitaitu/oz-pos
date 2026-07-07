import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Import modules fresh so we can mock window.matchMedia before the module-level mql is evaluated
let prefersReducedMotion: () => boolean;
let animDuration: (ms: number) => number;

async function loadModule(matches: boolean) {
  vi.doMock('@/utils/animation', () => {
    // We need to re-evaluate the module with our mock in place.
    // The simplest approach: re-import the original module.
    return import('@/utils/animation');
  });
  // Actually, we can't easily control the module-level mql this way.
  // Let's take a different approach.
}

// Since the module-level mql is cached at import time and we can't easily
// change it without vi.resetModules(), we test the default jsdom behavior
// (matchMedia returns matches: false) and document that reduced-motion
// testing requires vi.resetModules().

// We inline-import after module reset to test.
async function importFresh() {
  const mod = await import('@/utils/animation');
  return { prefersReducedMotion: mod.prefersReducedMotion, animDuration: mod.animDuration };
}

describe('prefersReducedMotion', () => {
  beforeEach(async () => {
    vi.resetModules();
    const mod = await importFresh();
    prefersReducedMotion = mod.prefersReducedMotion;
    animDuration = mod.animDuration;
  });

  it('returns false by default in jsdom', () => {
    // jsdom's test-setup.ts stubs matchMedia with matches: false
    expect(prefersReducedMotion()).toBe(false);
  });
});

describe('animDuration', () => {
  beforeEach(async () => {
    vi.resetModules();
    const mod = await importFresh();
    animDuration = mod.animDuration;
  });

  it('returns the given ms when motion is not reduced', () => {
    expect(animDuration(200)).toBe(200);
  });

  it('returns the given ms for arbitrary values', () => {
    expect(animDuration(500)).toBe(500);
    expect(animDuration(1)).toBe(1);
  });

  it('handles zero correctly', () => {
    expect(animDuration(0)).toBe(0);
  });
});

describe('animDuration with reduced motion', () => {
  it('returns 0 when prefers-reduced-motion is active', async () => {
    // Mock matchMedia to report reduced motion BEFORE importing the module
    const originalMatchMedia = window.matchMedia;
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: true,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(() => false),
    }));

    try {
      vi.resetModules();
      const mod = await import('@/utils/animation');
      // With reduced motion active, animDuration should return 0
      expect(mod.animDuration(200)).toBe(0);
      expect(mod.animDuration(500)).toBe(0);
      expect(mod.prefersReducedMotion()).toBe(true);
    } finally {
      window.matchMedia = originalMatchMedia;
    }
  });
});
