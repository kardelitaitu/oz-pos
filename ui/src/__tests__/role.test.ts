import { describe, it, expect } from 'vitest';
import { normalizeRole } from '../utils/role';

describe('normalizeRole', () => {
  it('returns owner for "owner"', () => {
    expect(normalizeRole('owner')).toBe('owner');
  });

  it('returns manager for "manager"', () => {
    expect(normalizeRole('manager')).toBe('manager');
  });

  it('returns cashier for "cashier"', () => {
    expect(normalizeRole('cashier')).toBe('cashier');
  });

  it('returns kitchen for "kitchen"', () => {
    expect(normalizeRole('kitchen')).toBe('kitchen');
  });

  it('returns kitchen for "kds" (alias)', () => {
    expect(normalizeRole('kds')).toBe('kitchen');
  });

  it('returns kitchen for "chef" (alias)', () => {
    expect(normalizeRole('chef')).toBe('kitchen');
  });

  it('returns staff for unknown role string', () => {
    expect(normalizeRole('superadmin')).toBe('staff');
  });

  it('returns staff for empty string', () => {
    expect(normalizeRole('')).toBe('staff');
  });

  it('returns staff for null', () => {
    expect(normalizeRole(null)).toBe('staff');
  });

  it('returns staff for undefined', () => {
    expect(normalizeRole(undefined)).toBe('staff');
  });

  it('trims whitespace', () => {
    expect(normalizeRole('  owner  ')).toBe('owner');
  });

  it('is case-insensitive', () => {
    expect(normalizeRole('OWNER')).toBe('owner');
    expect(normalizeRole('Manager')).toBe('manager');
    expect(normalizeRole('CASHIER')).toBe('cashier');
    expect(normalizeRole('Kitchen')).toBe('kitchen');
  });

  it('handles mixed case kds alias', () => {
    expect(normalizeRole('KDS')).toBe('kitchen');
  });
});
