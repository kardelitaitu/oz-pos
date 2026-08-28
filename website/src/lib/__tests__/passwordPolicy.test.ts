import { describe, expect, it } from 'vitest';
import { isStrongPassword, passwordsMatch } from '../passwordPolicy';

/**
 * Direct unit coverage of the client-side policy. The authoritative parity
 * gate is scripts/check-password-policy.mjs against the shared fixture
 * (password-policy-cases.json) — these pin the same rules for fast local
 * feedback.
 */
describe('password policy (mirrors the server gate)', () => {
  it('accepts a strong password', () => {
    expect(isStrongPassword('Abcdef12!')).toBe(true);
    expect(isStrongPassword('correct horse 9')).toBe(true);
  });

  it('rejects short, single-class, and whitespace-padded passwords', () => {
    expect(isStrongPassword('abcdefgh')).toBe(false); // one class
    expect(isStrongPassword('Abcdefg')).toBe(false); // < 8 chars
    expect(isStrongPassword('Abcdefg1 ')).toBe(false); // trailing space
    expect(isStrongPassword('Abcdefg1\t')).toBe(false); // trailing tab
    expect(isStrongPassword('')).toBe(false);
  });

  it('rejects byte overflow beyond the 72-byte cap even with enough classes', () => {
    // 30 emoji (4 bytes each) = 120 bytes — over the cap despite 3 classes.
    expect(isStrongPassword('A' + '😀'.repeat(30) + '1!')).toBe(false);
  });

  it('requires at least 3 of 4 character classes', () => {
    expect(isStrongPassword('abcdefg1')).toBe(false); // lower + digit only
    expect(isStrongPassword('Abcdef12')).toBe(true); // upper + lower + digit = 3
  });

  it('requires at least 8 runes even if byte length >= 8', () => {
    // 7 multi-byte characters (2 bytes each = 14 bytes) — rejected because < 8 runes
    expect(isStrongPassword('ÉÉÉÉ1a!')).toBe(false); // 7 runes (11 bytes) -> false
    expect(isStrongPassword('ÉÉÉ1a!')).toBe(false); // 6 runes (9 bytes) -> false
    expect(isStrongPassword('ÉÉÉÉÉ1a!')).toBe(true); // 8 runes (13 bytes) with upper + lower + digit + symbol
  });

  it('passwordsMatch requires non-empty identical values', () => {
    expect(passwordsMatch('Abcdef12!', 'Abcdef12!')).toBe(true);
    expect(passwordsMatch('Abcdef12!', 'Abcdef12?')).toBe(false);
    expect(passwordsMatch('', '')).toBe(false);
  });
});

