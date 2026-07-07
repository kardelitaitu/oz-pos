import { describe, expect, it } from 'vitest';
import { isSaleBarcode } from '@/utils/saleBarcode';

describe('isSaleBarcode', () => {
  it('returns true for a valid UUID', () => {
    expect(isSaleBarcode('a1b2c3d4-e5f6-7890-abcd-ef1234567890')).toBe(true);
  });

  it('returns true for uppercase UUID', () => {
    expect(isSaleBarcode('A1B2C3D4-E5F6-7890-ABCD-EF1234567890')).toBe(true);
  });

  it('trims whitespace before matching', () => {
    expect(isSaleBarcode('  a1b2c3d4-e5f6-7890-abcd-ef1234567890  ')).toBe(true);
  });

  it('returns false for non-UUID strings', () => {
    expect(isSaleBarcode('not-a-uuid')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isSaleBarcode('')).toBe(false);
  });

  it('returns false for gift card format', () => {
    expect(isSaleBarcode('GC-ABCDEF123456')).toBe(false);
  });

  it('returns false for random alphanumeric', () => {
    expect(isSaleBarcode('hello-world-1234')).toBe(false);
  });

  it('returns false for slightly-too-short UUID', () => {
    expect(isSaleBarcode('a1b2c3d4-e5f6-7890-abcd-ef12345678')).toBe(false);
  });
});
