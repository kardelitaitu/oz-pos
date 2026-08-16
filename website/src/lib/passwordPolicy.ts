/**
 * Client-side password policy — the SINGLE source of truth for the
 * website, mirrored 1:1 from the server's isValidPassword in
 * apps/license-server/web_password.go.
 *
 * Parity is enforced, not assumed: both sides are checked against
 * scripts/password-policy-cases.json —
 *   • the Go suite: TestPasswordPolicyMatchesSharedFixture
 *     (apps/license-server/web_password_test.go)
 *   • the website: scripts/check-password-policy.mjs (npm run
 *     precheck/prebuild), which imports THIS module and asserts every
 *     fixture case against it.
 * A change to the policy on either side without touching the fixture
 * fails the check on that side, so the meter and the server can never
 * drift apart silently.
 *
 * The rules (all mirrored):
 *   8–72 UTF-8 bytes, no leading/trailing whitespace, at least 8 runes
 *   (multi-byte passwords can't slip under the minimum by byte count),
 *   and at least 3 of the 4 character classes (lower / upper / digit /
 *   symbol).
 */

export const passwordMinLen = 8;
export const passwordMaxBytes = 72;
export const passwordMinClasses = 3;

const CLASS_RES: RegExp[] = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^A-Za-z0-9]/];

/** How many of the 4 character classes the password satisfies (0–4). */
export function passwordClassCount(password: string): number {
  return CLASS_RES.reduce((n, re) => (re.test(password) ? n + 1 : n), 0);
}

/** UTF-8 byte length — mirrors Go's `len(password)`. */
export function passwordByteLength(password: string): number {
  return new TextEncoder().encode(password).length;
}

/** Rune count — mirrors Go's `len([]rune(password))`. */
export function passwordRuneCount(password: string): number {
  return [...password].length;
}

/**
 * Whether the password satisfies the full policy — mirrors the server's
 * isValidPassword gate exactly. The meter's submit-disable uses this, so
 * the button enables on exactly the rule the server enforces.
 */
export function isStrongPassword(password: string): boolean {
  if (password === '') return false;
  if (
    passwordByteLength(password) < passwordMinLen ||
    passwordByteLength(password) > passwordMaxBytes
  ) {
    return false;
  }
  if (passwordRuneCount(password) < passwordMinLen) {
    return false;
  }
  if (password.trim() !== password) {
    return false;
  }
  return passwordClassCount(password) >= passwordMinClasses;
}

/**
 * Double-entry guard used by every create/change flow: the confirm field
 * must be non-empty and identical to the password before the submit
 * button enables (the server enforces the same rule via
 * password_confirm — see PasswordField and the handlers in
 * web_password.go).
 */
export function passwordsMatch(password: string, confirm: string): boolean {
  return password !== '' && password === confirm;
}
