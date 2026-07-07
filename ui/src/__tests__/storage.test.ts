import { describe, it, expect, vi, beforeEach } from 'vitest';
import { STORAGE_KEYS, getDecimalSep, setDecimalSep } from '@/utils/storage';

describe('STORAGE_KEYS', () => {
  it('defines expected keys', () => {
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

  it('returns dot when no value is stored', () => {
    expect(getDecimalSep()).toBe('dot');
  });

  it('returns stored value when valid', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'comma');
    expect(getDecimalSep()).toBe('comma');
  });

  it('returns dot when stored value is invalid', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'invalid');
    expect(getDecimalSep()).toBe('dot');
  });

  it('returns dot for stored empty string', () => {
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, '');
    expect(getDecimalSep()).toBe('dot');
  });

  it('returns all three valid separators', () => {
    expect(getDecimalSep()).toBe('dot');
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'comma');
    expect(getDecimalSep()).toBe('comma');
    localStorage.setItem(STORAGE_KEYS.DECIMAL_SEP, 'none');
    expect(getDecimalSep()).toBe('none');
  });
});

describe('setDecimalSep', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('persists a valid separator', () => {
    setDecimalSep('comma');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('comma');
  });

  it('does not persist an invalid separator', () => {
    setDecimalSep('invalid');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBeNull();
  });

  it('persists dot', () => {
    setDecimalSep('dot');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('dot');
  });

  it('persists none', () => {
    setDecimalSep('none');
    expect(localStorage.getItem(STORAGE_KEYS.DECIMAL_SEP)).toBe('none');
  });

  it('round-trips set then get', () => {
    setDecimalSep('comma');
    expect(getDecimalSep()).toBe('comma');
  });
});
