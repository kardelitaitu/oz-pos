import { describe, it, expect, beforeEach } from 'vitest';
import { STORAGE_KEYS, getDecimalSep, setDecimalSep } from '@/utils/storage';

describe('STORAGE_KEYS', () => {
  it('exposes CART_WIDTH, LOCKED_CART, LOCALE, and DECIMAL_SEP keys', () => {
    expect(STORAGE_KEYS.CART_WIDTH).toBe('pos-cart-width');
    expect(STORAGE_KEYS.LOCKED_CART).toBe('pos-locked-cart');
    expect(STORAGE_KEYS.LOCALE).toBe('oz-pos-locale');
    expect(STORAGE_KEYS.DECIMAL_SEP).toBe('oz-pos-decimal-sep');
  });
});

describe('getDecimalSep', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('returns "dot" by default when nothing is stored', () => {
    expect(getDecimalSep()).toBe('dot');
  });

  it('returns "dot" when stored value is "dot"', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'dot');
    expect(getDecimalSep()).toBe('dot');
  });

  it('returns "comma" when stored value is "comma"', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'comma');
    expect(getDecimalSep()).toBe('comma');
  });

  it('returns "none" when stored value is "none"', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'none');
    expect(getDecimalSep()).toBe('none');
  });

  it('ignores invalid stored values and falls back to "dot"', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'invalid');
    expect(getDecimalSep()).toBe('dot');
  });

  it('falls back to "dot" for empty string', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, '');
    expect(getDecimalSep()).toBe('dot');
  });
});

describe('setDecimalSep', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('persists "dot" to localStorage', () => {
    setDecimalSep('dot');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('dot');
  });

  it('persists "comma" to localStorage', () => {
    setDecimalSep('comma');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('comma');
  });

  it('persists "none" to localStorage', () => {
    setDecimalSep('none');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('none');
  });

  it('does not persist invalid values', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'dot');
    setDecimalSep('semicolon');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('dot');
  });

  it('round-trips: set then get returns the same value', () => {
    setDecimalSep('comma');
    expect(getDecimalSep()).toBe('comma');
    setDecimalSep('none');
    expect(getDecimalSep()).toBe('none');
    setDecimalSep('dot');
    expect(getDecimalSep()).toBe('dot');
  });
});
