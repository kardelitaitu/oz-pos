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
const ADMIN_KEYS = extractKeys(ADMIN_JS);
const LOGIN_KEYS = extractKeys(LOGIN_JS);

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