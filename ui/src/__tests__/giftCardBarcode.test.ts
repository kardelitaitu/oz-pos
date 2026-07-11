import { describe, it, expect } from 'vitest';
import { isGiftCardBarcode, generateGiftCardNumber } from '@/utils/giftCardBarcode';

// ── isGiftCardBarcode ───────────────────────────────────────────────

describe('isGiftCardBarcode', () => {
  it('returns true for valid 8-char code', () => {
    expect(isGiftCardBarcode('GC-ABCD1234')).toBe(true);
  });

  it('returns true for valid 16-char code', () => {
    expect(isGiftCardBarcode('GC-ABCDEF1234567890')).toBe(true);
  });

  it('returns true for valid 12-char code', () => {
    expect(isGiftCardBarcode('GC-ABC123DEF456')).toBe(true);
  });

  it('is case insensitive', () => {
    expect(isGiftCardBarcode('gc-abcd1234efgh')).toBe(true);
  });

  it('trims whitespace before testing', () => {
    expect(isGiftCardBarcode('  GC-ABCDEF1234  ')).toBe(true);
  });

  it('returns false when missing GC- prefix', () => {
    expect(isGiftCardBarcode('ABCDEF1234')).toBe(false);
  });

  it('returns false for too few chars (fewer than 8)', () => {
    expect(isGiftCardBarcode('GC-ABC123')).toBe(false);
  });

  it('returns false for too many chars (more than 16)', () => {
    expect(isGiftCardBarcode('GC-ABCDEF12345678901')).toBe(false);
  });

  it('returns false for special characters in code', () => {
    expect(isGiftCardBarcode('GC-ABCD@#$%EFGH')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isGiftCardBarcode('')).toBe(false);
  });

  it('returns false for whitespace-only string', () => {
    expect(isGiftCardBarcode('   ')).toBe(false);
  });
});

// ── generateGiftCardNumber ──────────────────────────────────────────

describe('generateGiftCardNumber', () => {
  it('starts with "GC-"', () => {
    const num = generateGiftCardNumber();
    expect(num.startsWith('GC-')).toBe(true);
  });

  it('has total length of 15 (GC- + 12 chars)', () => {
    const num = generateGiftCardNumber();
    expect(num.length).toBe(15);
  });

  it('only contains valid characters after prefix', () => {
    const num = generateGiftCardNumber();
    const code = num.slice(3); // after "GC-"
    expect(/^[A-Z0-9]{12}$/.test(code)).toBe(true);
  });

  it('generates different values on successive calls', () => {
    const results = new Set<string>();
    for (let i = 0; i < 10; i++) {
      results.add(generateGiftCardNumber());
    }
    expect(results.size).toBeGreaterThan(1);
  });
});
