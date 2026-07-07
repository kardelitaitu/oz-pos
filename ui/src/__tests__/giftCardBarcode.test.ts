import { describe, expect, it } from 'vitest';
import { isGiftCardBarcode, generateGiftCardNumber } from '@/utils/giftCardBarcode';

describe('isGiftCardBarcode', () => {
  it('returns true for valid GC- followed by 8 alphanumeric chars', () => {
    expect(isGiftCardBarcode('GC-ABCDEF12')).toBe(true);
  });

  it('returns true for valid GC- followed by 16 alphanumeric chars', () => {
    expect(isGiftCardBarcode('GC-ABCDEF1234567890')).toBe(true);
  });

  it('returns true for lowercase gc- prefix', () => {
    expect(isGiftCardBarcode('gc-abcdef123456')).toBe(true);
  });

  it('returns true for mixed case', () => {
    expect(isGiftCardBarcode('Gc-AbCd1234Ef56')).toBe(true);
  });

  it('trims whitespace before matching', () => {
    expect(isGiftCardBarcode('  GC-ABCDEF123456  ')).toBe(true);
  });

  it('returns false for codes without GC- prefix', () => {
    expect(isGiftCardBarcode('ABCDEF123456')).toBe(false);
  });

  it('returns false for too-short code (less than 8 chars after GC-)', () => {
    expect(isGiftCardBarcode('GC-ABCDEF')).toBe(false);
  });

  it('returns false for too-long code (more than 16 chars after GC-)', () => {
    expect(isGiftCardBarcode('GC-ABCDEF1234567890X')).toBe(false);
  });

  it('returns false for invalid characters after GC-', () => {
    expect(isGiftCardBarcode('GC-AB-DE-12-34')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isGiftCardBarcode('')).toBe(false);
  });

  it('returns false for UUID format', () => {
    expect(isGiftCardBarcode('a1b2c3d4-e5f6-7890-abcd-ef1234567890')).toBe(false);
  });
});

describe('generateGiftCardNumber', () => {
  it('returns a string in GC-XXXXXXXXXXXX format', () => {
    const result = generateGiftCardNumber();
    expect(result).toMatch(/^GC-[A-Z0-9]{12}$/);
  });

  it('generates unique numbers on successive calls', () => {
    const results = new Set<string>();
    for (let i = 0; i < 100; i++) {
      results.add(generateGiftCardNumber());
    }
    // With 100 calls and a 36^12 space, collisions are nearly impossible.
    expect(results.size).toBe(100);
  });

  it('passes the isGiftCardBarcode validator', () => {
    for (let i = 0; i < 20; i++) {
      expect(isGiftCardBarcode(generateGiftCardNumber())).toBe(true);
    }
  });
});
