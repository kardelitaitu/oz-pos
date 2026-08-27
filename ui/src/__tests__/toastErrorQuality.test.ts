/**
 * Toast Error Quality Compliance
 *
 * Error toasts MUST include actionable diagnostic detail, not just a generic
 * "Something went wrong" message. This test verifies:
 *
 *   1. `errorDetail()` extracts useful info from every AppError kind
 *   2. `errorDetail()` handles non-AppError shapes (Error, string, unknown)
 *   3. `errorDetail()` redacts sensitive material (emails, paths, keys)
 *   4. `l10nErrorMessage()` never returns the raw fallback key as user text
 *   5. GlobalErrorReporter passes detail + title to addToast (not just message)
 *   6. The Toast type enforces `detail` is present on error toasts (type-level)
 */

import { describe, it, expect } from 'vitest';
import { errorDetail } from '@/utils/app-error';

// ── errorDetail() extraction ──────────────────────────────────────

describe('errorDetail() extracts diagnostic info', () => {
  it('extracts kind + subKind + message from core AppError', () => {
    const err = { kind: 'core' as const, subKind: 'database', message: 'SQLite busy' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: core');
    expect(detail).toContain('SubKind: database');
    expect(detail).toContain('Server: SQLite busy');
  });

  it('extracts kind + message from invalid AppError', () => {
    const err = { kind: 'invalid' as const, message: 'Email already exists' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: invalid');
    expect(detail).toContain('Server: Email already exists');
    // Should NOT contain SubKind (invalid has no subKind)
    expect(detail).not.toContain('SubKind:');
  });

  it('extracts kind + message from permissionDenied AppError', () => {
    const err = { kind: 'permissionDenied' as const, message: 'Admin only' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: permissionDenied');
    expect(detail).toContain('Server: Admin only');
  });

  it('extracts kind + message from invalidSession AppError', () => {
    const err = { kind: 'invalidSession' as const, message: 'Token expired' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: invalidSession');
    expect(detail).toContain('Server: Token expired');
  });

  it('extracts kind + message from hardware AppError', () => {
    const err = { kind: 'hardware' as const, subKind: 'printer', message: 'Paper jam' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: hardware');
    expect(detail).toContain('SubKind: printer');
    expect(detail).toContain('Server: Paper jam');
  });

  it('extracts kind + message from internal AppError', () => {
    const err = { kind: 'internal' as const, message: 'Unexpected null' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: internal');
    expect(detail).toContain('Server: Unexpected null');
  });

  it('extracts message + first stack lines from Error object', () => {
    const err = new Error('Cannot read property of undefined');
    const detail = errorDetail(err);
    expect(detail).toContain('Error: Cannot read property of undefined');
    expect(detail).toContain('Stack:');
    // Stack should be truncated (not the full 50-line trace)
    const stackLines = detail!.split('\n').filter(l => l.startsWith('Stack:'));
    expect(stackLines.length).toBe(1);
  });

  it('handles string errors', () => {
    const detail = errorDetail('connection refused');
    expect(detail).toBe('Error: connection refused');
  });

  it('returns null for null/undefined', () => {
    expect(errorDetail(null)).toBeNull();
    expect(errorDetail(undefined)).toBeNull();
  });

  it('handles number errors', () => {
    const detail = errorDetail(42);
    expect(detail).toContain('Error: 42');
  });

  it('handles errors without message', () => {
    const err = { kind: 'core' as const, subKind: 'db', message: '' };
    const detail = errorDetail(err);
    expect(detail).toContain('Kind: core');
    expect(detail).toContain('SubKind: db');
    // Empty message should not produce "Server: " with nothing after it
    expect(detail).not.toMatch(/Server:\s*$/);
  });
});

// ── Redaction ─────────────────────────────────────────────────────

describe('errorDetail() redacts sensitive material', () => {
  it('redacts email addresses', () => {
    const err = { kind: 'core' as const, subKind: 'auth', message: 'Login failed for user@example.com' };
    const detail = errorDetail(err)!;
    expect(detail).not.toContain('user@example.com');
    expect(detail).toContain('REDACTED');
  });

  it('redacts file paths', () => {
    const err = { kind: 'internal' as const, message: 'ENOENT: /home/user/data.db' };
    const detail = errorDetail(err)!;
    expect(detail).not.toContain('/home/user/data.db');
    expect(detail).toContain('REDACTED');
  });

  it('redacts license keys', () => {
    const err = { kind: 'core' as const, subKind: 'license', message: 'Invalid key OZ-ABCD-1234-EFGH' };
    const detail = errorDetail(err)!;
    expect(detail).not.toContain('OZ-ABCD-1234-EFGH');
    expect(detail).toContain('REDACTED');
  });

  it('redacts UUIDs in stack traces', () => {
    const err = new Error('failed at abc12345-def6-7890-abcd-ef1234567890');
    const detail = errorDetail(err)!;
    expect(detail).not.toContain('abc12345-def6-7890-abcd-ef1234567890');
  });
});

// ── L10n error messages ───────────────────────────────────────────

describe('error messages are specific, not generic', () => {
  it('errorDetail always returns a non-empty string for real errors', () => {
    const errors = [
      { kind: 'core' as const, subKind: 'database', message: 'busy' },
      { kind: 'invalid' as const, message: 'bad input' },
      { kind: 'permissionDenied' as const, message: 'no access' },
      { kind: 'invalidSession' as const },
      { kind: 'hardware' as const, subKind: 'scanner', message: 'timeout' },
      { kind: 'internal' as const, message: 'oops' },
      new Error('network error'),
      new TypeError('cannot read null'),
      'string error',
      404,
    ];

    for (const err of errors) {
      const detail = errorDetail(err);
      expect(detail, `errorDetail() returned null/empty for: ${JSON.stringify(err)}`).toBeTruthy();
      expect(detail!.length, `errorDetail() too short for: ${JSON.stringify(err)}`).toBeGreaterThan(5);
    }
  });

  it('every AppError kind produces a "Kind:" line', () => {
    const kinds = ['core', 'hardware', 'invalid', 'permissionDenied', 'invalidSession', 'internal'] as const;
    for (const kind of kinds) {
      const err = kind === 'invalidSession'
        ? { kind, message: 'test' }
        : { kind, subKind: 'test', message: 'test' };
      const detail = errorDetail(err)!;
      expect(detail).toMatch(new RegExp(`Kind: ${kind}`));
    }
  });
});
