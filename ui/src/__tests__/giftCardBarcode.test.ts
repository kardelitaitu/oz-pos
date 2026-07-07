import { describe, it, expect } from 'vitest';
import { isGiftCardBarcode, generateGiftCardNumber } from '@/utils/giftCardBarcode';

describe('isGiftCardBarcode', () => {
  it('matches a valid gift card barcode', () => {
    expect(isGiftCardBarcode('GC-ABCDEF123456')).toBe(true);
  });

  it('matches lowercase prefix', () => {
    expect(isGiftCardBarcode('gc-abcdef123456')).toBe(true);
  });

  it('matches with surrounding whitespace', () => {
    expect(isGiftCardBarcode('  GC-ABCDEF123456  ')).toBe(true);
  });

  it('matches minimum length (8 chars)', () => {
    expect(isGiftCardBarcode('GC-12345678')).toBe(true);
  });

  it('matches maximum length (16 chars)', () => {
    expect(isGiftCardBarcode('GC-ABCDEFGH12345678')).toBe(true);
  });

  it('rejects missing prefix', () => {
    expect(isGiftCardBarcode('ABCDEF123456')).toBe(false);
  });

  it('rejects too few chars after prefix', () => {
    expect(isGiftCardBarcode('GC-1234567')).toBe(false);
  });

  it('rejects too many chars after prefix', () => {
    expect(isGiftCardBarcode('GC-12345678901234567')).toBe(false);
  });

  it('accepts lowercase letters (regex has /i flag)', () => {
    expect(isGiftCardBarcode('gc-abcdefabcdef')).toBe(true);
  });

  it('rejects special characters', () => {
    expect(isGiftCardBarcode('GC-ABCD-EF123456')).toBe(false);
  });

  it('rejects empty string', () => {
    expect(isGiftCardBarcode('')).toBe(false);
  });
});

describe('generateGiftCardNumber', () => {
  it('generates a string starting with GC-', () => {
    const num = generateGiftCardNumber();
    expect(num.startsWith('GC-')).toBe(true);
  });

  it('generates a 15-character string (GC- + 12)', () => {
    const num = generateGiftCardNumber();
    expect(num).toHaveLength(15);
  });

  it('generates a valid gift card barcode', () => {
    const num = generateGiftCardNumber();
    expect(isGiftCardBarcode(num)).toBe(true);
  });

  it('generates different numbers on subsequent calls', () => {
    // With 36^12 possible values, collisions are astronomically unlikely
    const a = generateGiftCardNumber();
    const b = generateGiftCardNumber();
    expect(a).not.toBe(b);
  });
});
