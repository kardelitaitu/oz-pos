import { describe, it, expect } from 'vitest';
import { isSaleBarcode } from '@/utils/saleBarcode';

describe('isSaleBarcode', () => {
  it('returns true for valid lowercase UUID', () => {
    expect(isSaleBarcode('a1b2c3d4-e5f6-7890-abcd-ef1234567890')).toBe(true);
  });

  it('returns true for valid uppercase UUID', () => {
    expect(isSaleBarcode('A1B2C3D4-E5F6-7890-ABCD-EF1234567890')).toBe(true);
  });

  it('returns true for mixed-case UUID', () => {
    expect(isSaleBarcode('A1b2C3d4-E5f6-7890-AbCd-Ef1234567890')).toBe(true);
  });

  it('trims whitespace before testing', () => {
    expect(isSaleBarcode('  a1b2c3d4-e5f6-7890-abcd-ef1234567890  ')).toBe(true);
  });

  it('returns false for non-UUID format', () => {
    expect(isSaleBarcode('hello-world')).toBe(false);
  });

  it('returns false for too few characters', () => {
    expect(isSaleBarcode('abc-123-def')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isSaleBarcode('')).toBe(false);
  });

  it('returns false for whitespace-only string', () => {
    expect(isSaleBarcode('   ')).toBe(false);
  });
});
