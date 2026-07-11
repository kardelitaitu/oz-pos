import { describe, it, expect, vi } from 'vitest';

// animation.ts reads window.matchMedia at module top-level, so we must
// set up the mock BEFORE importing. Use Object.defineProperty + top-level
// await to ensure matchMedia is in place when mql is initialised.

let _matches = false;

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  configurable: true,
  value: vi.fn().mockImplementation(() => ({
    // getter so mql.matches dynamically reads current _matches value
    get matches() { return _matches; },
    media: '(prefers-reduced-motion: reduce)',
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })),
});

function setMatches(v: boolean) {
  _matches = v;
}

const { prefersReducedMotion, animDuration } = await import('@/utils/animation');

describe('prefersReducedMotion', () => {
  it('returns false when user has not requested reduced motion', () => {
    setMatches(false);
    expect(prefersReducedMotion()).toBe(false);
  });

  it('returns true when user has requested reduced motion', () => {
    setMatches(true);
    expect(prefersReducedMotion()).toBe(true);
  });
});

describe('animDuration', () => {
  it('returns the given ms when reduced motion is not preferred', () => {
    setMatches(false);
    expect(animDuration(200)).toBe(200);
    expect(animDuration(500)).toBe(500);
  });

  it('returns 0 when reduced motion is preferred', () => {
    setMatches(true);
    expect(animDuration(200)).toBe(0);
    expect(animDuration(500)).toBe(0);
  });

  it('returns 0 for very large values when reduced motion is preferred', () => {
    setMatches(true);
    expect(animDuration(10000)).toBe(0);
  });

  it('preserves exact ms when not reduced motion even at 0', () => {
    setMatches(false);
    expect(animDuration(0)).toBe(0);
  });
});
