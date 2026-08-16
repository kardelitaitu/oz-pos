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

  it('recognises admin', () => {
    expect(normalizeRole('admin')).toBe('admin');
  });

  it('recognises manager', () => {
    expect(normalizeRole('manager')).toBe('manager');
  });

  it('recognises staff', () => {
    expect(normalizeRole('staff')).toBe('staff');
  });

  it('recognises auditor', () => {
    expect(normalizeRole('auditor')).toBe('auditor');
  });

  it('recognises role-prefixed preset ids', () => {
    expect(normalizeRole('role-owner')).toBe('owner');
    expect(normalizeRole('role-admin')).toBe('admin');
    expect(normalizeRole('role-manager')).toBe('manager');
    expect(normalizeRole('role-staff')).toBe('staff');
    expect(normalizeRole('role-auditor')).toBe('auditor');
  });

  it('falls back to staff for retired cashier', () => {
    expect(normalizeRole('cashier')).toBe('staff');
    expect(normalizeRole('role-cashier')).toBe('staff');
  });

  it('falls back to staff for retired kitchen and its aliases', () => {
    expect(normalizeRole('kitchen')).toBe('staff');
    expect(normalizeRole('role-kitchen')).toBe('staff');
    expect(normalizeRole('kds')).toBe('staff');
    expect(normalizeRole('chef')).toBe('staff');
  });

  it('falls back to staff for unknown roles', () => {
    expect(normalizeRole('administrator')).toBe('staff');
    expect(normalizeRole('supervisor')).toBe('staff');
    expect(normalizeRole('waiter')).toBe('staff');
  });

  it('is case-insensitive', () => {
    expect(normalizeRole('OWNER')).toBe('owner');
    expect(normalizeRole('Admin')).toBe('admin');
    expect(normalizeRole('Manager')).toBe('manager');
    expect(normalizeRole('STAFF')).toBe('staff');
    expect(normalizeRole('Auditor')).toBe('auditor');
    expect(normalizeRole('CASHIER')).toBe('staff');
  });

  it('trims whitespace', () => {
    expect(normalizeRole('  owner  ')).toBe('owner');
    expect(normalizeRole('\tmanager\n')).toBe('manager');
  });

  it('handles mixed case with whitespace', () => {
    expect(normalizeRole('  AuDiToR  ')).toBe('auditor');
  });
});
