// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import { clearSession, EMAIL_KEY, hasSession, isPlaceholderPriceId, SESSION_KEY } from '../paddle';

/**
 * Session storage helpers. The critical regression: clearSession must
 * remove the cached email WITH the token — otherwise the next account on
 * the same browser gets the previous user's email prefilled in Paddle
 * checkout, attaching the subscription to the wrong tenant.
 */
describe('session helpers', () => {
  beforeEach(() => sessionStorage.clear());

  it('clearSession removes BOTH the token and the cached email', () => {
    sessionStorage.setItem(SESSION_KEY, 'tok');
    sessionStorage.setItem(EMAIL_KEY, 'alice@example.com');
    clearSession();
    expect(sessionStorage.getItem(SESSION_KEY)).toBeNull();
    expect(sessionStorage.getItem(EMAIL_KEY)).toBeNull();
  });

  it('clearSession is a no-op when storage is empty', () => {
    expect(() => clearSession()).not.toThrow();
  });

  it('hasSession reflects the stored token', () => {
    expect(hasSession()).toBe(false);
    sessionStorage.setItem(SESSION_KEY, 'tok');
    expect(hasSession()).toBe(true);
    sessionStorage.removeItem(SESSION_KEY);
    expect(hasSession()).toBe(false);
  });

  it('isPlaceholderPriceId detects only placeholder ids', () => {
    expect(isPlaceholderPriceId('pri_placeholder_x')).toBe(true);
    expect(isPlaceholderPriceId('pri_01m05gdnqp30xze6db73qcracp')).toBe(false);
    expect(isPlaceholderPriceId(undefined)).toBe(false);
  });
});
