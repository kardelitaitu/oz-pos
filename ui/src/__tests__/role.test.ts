import { describe, expect, it } from 'vitest';
import { normalizeRole } from '@/utils/role';

describe('normalizeRole', () => {
  it('returns staff for null', () => {
    expect(normalizeRole(null)).toBe('staff');
  });

  it('returns staff for undefined', () => {
    expect(normalizeRole(undefined)).toBe('staff');
  });

  it('returns staff for empty string', () => {
    expect(normalizeRole('')).toBe('staff');
  });

  it('returns staff for whitespace-only', () => {
    expect(normalizeRole('   ')).toBe('staff');
  });

  it('recognises owner', () => {
    expect(normalizeRole('owner')).toBe('owner');
  });

  it('recognises manager', () => {
    expect(normalizeRole('manager')).toBe('manager');
  });

  it('recognises cashier', () => {
    expect(normalizeRole('cashier')).toBe('cashier');
  });

  it('recognises kitchen', () => {
    expect(normalizeRole('kitchen')).toBe('kitchen');
  });

  it('maps kds alias to kitchen', () => {
    expect(normalizeRole('kds')).toBe('kitchen');
  });

  it('maps chef alias to kitchen', () => {
    expect(normalizeRole('chef')).toBe('kitchen');
  });

  it('falls back to staff for unknown roles', () => {
    expect(normalizeRole('administrator')).toBe('staff');
    expect(normalizeRole('supervisor')).toBe('staff');
    expect(normalizeRole('waiter')).toBe('staff');
  });

  it('is case-insensitive', () => {
    expect(normalizeRole('OWNER')).toBe('owner');
    expect(normalizeRole('Manager')).toBe('manager');
    expect(normalizeRole('CASHIER')).toBe('cashier');
    expect(normalizeRole('Kitchen')).toBe('kitchen');
    expect(normalizeRole('KDS')).toBe('kitchen');
    expect(normalizeRole('Chef')).toBe('kitchen');
  });

  it('trims whitespace', () => {
    expect(normalizeRole('  owner  ')).toBe('owner');
    expect(normalizeRole('\tmanager\n')).toBe('manager');
  });

  it('handles mixed case with whitespace', () => {
    expect(normalizeRole('  CaShIeR  ')).toBe('cashier');
  });
});
