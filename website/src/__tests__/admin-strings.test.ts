// @vitest-environment jsdom
// Admin i18n contract — every t('key') call in the admin/login scripts must
// resolve in the STRINGS table. A typo in admin.js or login.js would
// otherwise silently render the literal key text (STRINGS.t falls back to
// the key), and the website i18n audit (scripts/audit-i18n.mjs) does not
// scan the admin STRINGS (it's a separate system from src/i18n).
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import utils from '../../public/admin/admin-utils.js';

const STRING_KEYS: Set<string> = new Set(Object.keys(utils.STRINGS));

/** Extract every t('literal') key from source text. */
function extractKeys(src: string): string[] {
  const keys = new Set<string>();
  // Drop // line comments first — they can contain t('…') as prose (e.g.
  // the B2 comment "every t('…') label below threw TypeError"), which is
  // not a real i18n call and must not count as a missing key.
  const code = src.replace(/\/\/[^\n]*/g, '');
  const re = /t\(\s*(['"])([^'"]+)\1\s*\)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(code)) !== null) keys.add(m[2]);
  return [...keys];
}

const ADMIN_JS = readFileSync(join('public/admin', 'admin.js'), 'utf8');
const LOGIN_JS = readFileSync(join('public/admin', 'login.js'), 'utf8');
const UTILS_JS = readFileSync(join('public/admin', 'admin-utils.js'), 'utf8');
const ADMIN_KEYS = extractKeys(ADMIN_JS);
const LOGIN_KEYS = extractKeys(LOGIN_JS);
const UTILS_KEYS = extractKeys(UTILS_JS);

describe('admin.js STRINGS coverage', () => {
  it('every t(\'key\') call in admin.js exists in STRINGS', () => {
    const missing = ADMIN_KEYS.filter((k) => !STRING_KEYS.has(k));
    expect(missing, 'missing keys in admin.js').toEqual([]);
  });
});

describe('login.js STRINGS coverage', () => {
  it('every t(\'key\') call in login.js exists in STRINGS', () => {
    const missing = LOGIN_KEYS.filter((k) => !STRING_KEYS.has(k));
    expect(missing, 'missing keys in login.js').toEqual([]);
  });
});

describe('admin-utils.js STRINGS coverage', () => {
  it('every t(\'key\') call in admin-utils.js exists in STRINGS', () => {
    const missing = UTILS_KEYS.filter((k) => !STRING_KEYS.has(k));
    expect(missing, 'missing keys in admin-utils.js').toEqual([]);
  });
});

describe('admin STRINGS purity (P3: no HTML-in-i18n)', () => {
  it('no STRINGS value embeds HTML markup', () => {
    // The sign-in-again message used to carry an <a> tag injected via
    // innerHTML — an XSS footgun. Values must be plain text; any link or
    // structure belongs in the DOM-building code, not the i18n table.
    const htmlish = Object.keys(utils.STRINGS).filter((k) =>
      /<[a-z][^>]*>|&[a-z]+;|on\w+=/i.test(String(utils.STRINGS[k as keyof typeof utils.STRINGS])),
    );
    expect(htmlish, 'STRINGS values containing HTML markup').toEqual([]);
  });

  it('legacy auth.signInAgain split into text-only parts, no leftover reference', () => {
    expect(STRING_KEYS.has('auth.signInAgain')).toBe(false);
    expect(STRING_KEYS.has('auth.signInAgainBefore')).toBe(true);
    expect(STRING_KEYS.has('auth.signInAgainLink')).toBe(true);
    expect(STRING_KEYS.has('auth.signInAgainAfter')).toBe(true);
    // admin.js must no longer string-concat innerHTML with the old key.
    expect(ADMIN_JS).not.toContain("'auth.signInAgain'");
    expect(ADMIN_JS).toContain("t('auth.signInAgainBefore')");
  });
});