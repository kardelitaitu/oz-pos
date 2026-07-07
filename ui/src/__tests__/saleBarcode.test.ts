import { describe, it, expect } from 'vitest';
import { isSaleBarcode } from '@/utils/saleBarcode';

describe('isSaleBarcode', () => {
  it('matches a valid UUID', () => {
    expect(isSaleBarcode('550e8400-e29b-41d4-a716-446655440000')).toBe(true);
  });

  it('matches with surrounding whitespace', () => {
    expect(isSaleBarcode('  550e8400-e29b-41d4-a716-446655440000  ')).toBe(true);
  });

  it('matches uppercase UUID', () => {
    expect(isSaleBarcode('550E8400-E29B-41D4-A716-446655440000')).toBe(true);
  });

  it('matches mixed case UUID', () => {
    expect(isSaleBarcode('550e8400-E29B-41d4-a716-446655440000')).toBe(true);
  });

  it('rejects a random string', () => {
    expect(isSaleBarcode('not-a-uuid')).toBe(false);
  });

  it('rejects a gift card barcode', () => {
    // Gift cards start with GC-, which is not a UUID
    expect(isSaleBarcode('GC-ABCDEF123456')).toBe(false);
  });

  it('rejects short UUID-like string (7 hyphens groups)', () => {
    // UUID has exactly 8-4-4-4-12 format, extra segments fail
    expect(isSaleBarcode('550e8400-e29b-41d4-a716-446655440000-extra')).toBe(false);
  });

  it('rejects empty string', () => {
    expect(isSaleBarcode('')).toBe(false);
  });

  it('rejects a product SKU string', () => {
    expect(isSaleBarcode('PROD-001')).toBe(false);
  });
});
