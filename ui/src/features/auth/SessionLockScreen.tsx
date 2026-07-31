import { useState, useEffect, useCallback, useRef } from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { useAuth } from '@/contexts/AuthContext';
import { useSyncConnection } from '@/hooks/useSyncConnection';
import { checkLicenseStatus } from '@/api/license';
import { staffLogin } from '@/api/staff';
import { useLocalization } from '@fluent/react';
import './SessionLockScreen.css';

const MAX_PIN_LENGTH = 4;
const MAX_PIN_ATTEMPTS = 5;
const LOCKOUT_DURATION_MS = 30_000;

function AlertIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}

/**
 * Session lock screen — shown after idle timeout.
 * Displays current time, a "Session Locked" message, and
 * a PIN pad for re-entry. On successful PIN match, calls
 * `onUnlock`. On failure, shows error.
 */
export default function SessionLockScreen({
  onUnlock,
}: {
  onUnlock: () => void;
}) {
  const { session } = useAuth();
  const { l10n } = useLocalization();
  const [pin, setPin] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [time, setTime] = useState(new Date());
  const [lockedUntil, setLockedUntil] = useState<number | null>(null);
  const [pinAttempts, setPinAttempts] = useState(0);
  const pinWrapRef = useRef<HTMLDivElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const lastErrorRef = useRef<string | null>(null);

  const syncStatus = useSyncConnection();
  const [authOnline, setAuthOnline] = useState<boolean | null>(null);
  const [authLatency, setAuthLatency] = useState<number | null>(null);

  // Check license once on mount (decorative status pill — no continuous polling needed).
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const start = performance.now();
        const result = await checkLicenseStatus();
        if (!mounted) return;
        if (result.active) {
          setAuthLatency(Math.round(performance.now() - start));
          setAuthOnline(true);
        } else {
          setAuthLatency(null);
          setAuthOnline(false);
        }
      } catch {
        if (!mounted) return;
        setAuthLatency(null);
        setAuthOnline(false);
      }
    })();
    return () => { mounted = false; };
  }, []);

  // Auto-unlock after lockout period
  useEffect(() => {
    if (lockedUntil === null) return;
    const remaining = lockedUntil - Date.now();
    if (remaining <= 0) {
      setLockedUntil(null);
      return;
    }
    const timer = setTimeout(() => {
      setLockedUntil(null);
    }, remaining);
    return () => clearTimeout(timer);
  }, [lockedUntil]);

  const isLocked = lockedUntil !== null;
  const lockoutRemainingSec = lockedUntil !== null
    ? Math.max(0, Math.ceil((lockedUntil - Date.now()) / 1000))
    : 0;

  // Shake card on error
  useEffect(() => {
    if (!error || error === lastErrorRef.current) return;
    lastErrorRef.current = error;

    const card = cardRef.current;
    if (card) {
      card.classList.add('session-lock-card--shake');
      setTimeout(() => card.classList.remove('session-lock-card--shake'), 350);
    }
  }, [error]);

  // Update clock every 30s
  useEffect(() => {
    const id = setInterval(() => setTime(new Date()), 30_000);
    return () => clearInterval(id);
  }, []);

  // Focus PIN area on mount
  useEffect(() => {
    pinWrapRef.current?.focus();
  }, []);

  const handleDigit = useCallback((digit: string) => {
    if (isLocked) return;
    setError('');
    setPin((prev) => (prev.length >= MAX_PIN_LENGTH ? prev : [...prev, digit]));
  }, [isLocked]);

  const handleBackspace = useCallback(() => {
    if (isLocked) return;
    setPin((prev) => prev.slice(0, -1));
  }, [isLocked]);

  // Auto-submit on 4 digits — verify PIN via staffLogin
  useEffect(() => {
    if (pin.length !== MAX_PIN_LENGTH) return;
    const entered = pin.join('');
    if (!session) return;

    const attemptLogin = async () => {
      try {
        const username = sessionStorage.getItem('current-username');
        if (!username) {
          setError(l10n.getString('session-lock-expired'));
          setPin([]);
          return;
        }
        await staffLogin({ username, pin: entered });
        // Success — PIN verified
        setPin([]);
        setPinAttempts(0);
        onUnlock();
      } catch (err) {
        const errObj = err as Record<string, unknown> | null;
        const msg = errObj?.['message'] as string
          ?? l10n.getString('session-lock-invalid-pin');
        setError(msg);
        setPin([]);

        // Increment local attempt counter (defense-in-depth).
        const nextAttempts = pinAttempts + 1;
        setPinAttempts(nextAttempts);

        // Enforce local lockout after MAX_PIN_ATTEMPTS.
        if (nextAttempts >= MAX_PIN_ATTEMPTS) {
          setLockedUntil(Date.now() + LOCKOUT_DURATION_MS);
        }

        // Also respect backend rate-limit instructions, if present.
        const lockoutSeconds = parseRateLimitSeconds(msg, errObj);
        if (lockoutSeconds !== null) {
          setLockedUntil(Date.now() + lockoutSeconds * 1000);
        }
      }
    };
    attemptLogin();
  }, [pin, session, onUnlock, l10n, pinAttempts]);

  /**
   * Parse the retry-after duration from a rate-limit error, supporting
   * both the legacy string pattern and a future structured field.
   */
  function parseRateLimitSeconds(
    msg: string,
    errObj: Record<string, unknown> | null,
  ): number | null {
    // 1. Structured field on the error object (future-proof).
    const retryAfter = errObj?.['retry_after_seconds'] ?? errObj?.['retry-after'];
    if (typeof retryAfter === 'number' && Number.isFinite(retryAfter) && retryAfter > 0) {
      return Math.ceil(retryAfter);
    }
    // 2. Legacy string pattern: "Try again in Ns" or "Try again in N seconds".
    const match = msg.match(/Try again in (\d+)\s*s(?:econds?)?/i);
    if (match && match[1]) {
      return parseInt(match[1], 10);
    }
    return null;
  }

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key >= '0' && e.key <= '9') {
        e.preventDefault();
        handleDigit(e.key);
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        handleBackspace();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setPin([]);
      }
    },
    [handleDigit, handleBackspace],
  );

  const timeStr = time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const dateStr = time.toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
  });

  return (
    <div className="session-lock-overlay">
      <div className="session-lock-backdrop" />
      <div className="session-lock-card" ref={cardRef}>
        {/* Lock icon */}
        <div className="session-lock-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="32" height="32">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
        </div>

        {/* Time */}
        <div className="session-lock-time">{timeStr}</div>
        <div className="session-lock-date">{dateStr}</div>

        <div className="session-lock-sub">{requiredLocalized(l10n, 'session-lock-enter-pin')}</div>

        {/* PIN dots */}
        <div className="session-lock-pin-dots" aria-label={requiredLocalized(l10n, 'session-lock-pin-aria', { length: String(pin.length), max: String(MAX_PIN_LENGTH) })}>
          {Array.from({ length: MAX_PIN_LENGTH }, (_, i) => (
            <span
              key={i}
              className={`session-lock-pin-dot ${i < pin.length ? 'session-lock-pin-dot--filled' : ''}`}
            />
          ))}
        </div>

        {/* Error */}
        {error && (
          <div className="session-lock-error" role="alert" aria-live="polite">
            <AlertIcon />
            {error}
            {isLocked && (
              <span className="session-lock-rate-limit">
                {' '}{requiredLocalized(l10n, 'session-lock-lockout', { seconds: String(lockoutRemainingSec) })}
              </span>
            )}
          </div>
        )}

        {/* PIN pad */}
        {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
        <div
          id="session-lock-pin-pad"
          className="session-lock-pad"
          ref={pinWrapRef}
          tabIndex={-1}
          onKeyDown={handleKeyDown}
          role="application"
          aria-label={requiredLocalized(l10n, 'session-lock-pad-aria')}
        >
          {[['7', '8', '9'], ['4', '5', '6'], ['1', '2', '3']].map((row) => (
            <div className="session-lock-pad-row" key={row[0]}>
              {row.map((digit) => (
                <button
                  key={digit}
                  type="button"
                  className="session-lock-pad-key"
                  onClick={() => handleDigit(digit)}
                  aria-label={digit}
                  disabled={isLocked}
                >
                  {digit}
                </button>
              ))}
            </div>
          ))}
          <div className="session-lock-pad-row">
            <button
              type="button"
              className="session-lock-pad-key session-lock-pad-key--action"
              onClick={() => setPin([])}
              disabled={pin.length === 0 || isLocked}
            >
              {requiredLocalized(l10n, 'staff-login-clear')}
            </button>
            <button
              type="button"
              className="session-lock-pad-key"
              onClick={() => handleDigit('0')}
              aria-label="0"
              disabled={isLocked}
            >
              0
            </button>
            <button
              type="button"
              className="session-lock-pad-key session-lock-pad-key--action"
              onClick={handleBackspace}
              disabled={pin.length === 0 || isLocked}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="18" height="18">
                <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z" />
                <line x1="18" y1="9" x2="12" y2="15" />
                <line x1="12" y1="9" x2="18" y2="15" />
              </svg>
            </button>
          </div>
        </div>

        {/* ── Connection status indicators ──────── */}
        <div className="session-lock-connection-group">
          {/* Auth status — via checkLicenseStatus IPC */}            <div className="connection-status" title={authOnline === null ? requiredLocalized(l10n, 'staff-login-connection-checking') : authOnline ? requiredLocalized(l10n, 'staff-login-connection-connected') : requiredLocalized(l10n, 'staff-login-connection-disconnected')}>
            <span className={`status-indicator ${authOnline === null ? 'checking' : authOnline ? 'online' : 'offline'}`} />
            <span className="connection-label">{requiredLocalized(l10n, 'staff-login-connection-auth')}</span>
            {authOnline && authLatency !== null && <span className="connection-latency">{authLatency}ms</span>}
          </div>
          {/* Sync status — via useSyncConnection IPC */}            <div className="connection-status" title={syncStatus.state === 'checking' ? requiredLocalized(l10n, 'staff-login-connection-checking') : syncStatus.state === 'connected' ? requiredLocalized(l10n, 'staff-login-connection-connected') : requiredLocalized(l10n, 'staff-login-connection-disconnected')}>
            <span className={`status-indicator ${syncStatus.state === 'checking' ? 'checking' : syncStatus.state === 'connected' ? 'online' : 'offline'}`} />
            <span className="connection-label">{requiredLocalized(l10n, 'staff-login-connection-sync')}</span>
            {syncStatus.state === 'connected' && syncStatus.latencyMs !== null && <span className="connection-latency">{syncStatus.latencyMs}ms</span>}
          </div>
        </div>
      </div>
    </div>
  );
}
