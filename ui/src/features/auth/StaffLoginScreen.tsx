import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { checkUsername } from '@/api/staff';
import { useAuth } from '@/contexts/AuthContext';
import { useBrand } from '@/contexts/BrandContext';
import { useToast } from '@/frontend/shared/Toast';
import { Localized } from '@/frontend/shared/Localized';
import { useLocalization } from '@fluent/react';
import { convertFileSrc } from '@tauri-apps/api/core';
import './StaffLoginScreen.css';

// ── SVG icons ───────────────────────────────────────────────────────

function UserIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
      <circle cx="12" cy="7" r="4" />
    </svg>
  );
}

function BackspaceIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z" />
      <line x1="18" y1="9" x2="12" y2="15" />
      <line x1="12" y1="9" x2="18" y2="15" />
    </svg>
  );
}

function AlertIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}

// ── Constants ───────────────────────────────────────────────────────

const MAX_PIN_LENGTH = 4;
const MAX_PIN_ATTEMPTS = 5;
const RATE_LIMIT_WARN_AFTER = 3;

// ── Resized Logo Helper (1:1 PNG at target size) ───────────────────

function useResizedLogo(src: string | null | undefined, targetSize = 256) {
  const [resizedUrl, setResizedUrl] = useState<string | null>(src || null);
  const [error, setError] = useState(false);

  useEffect(() => {
    if (!src) {
      setResizedUrl(null);
      setError(false);
      return;
    }
    setResizedUrl(src);
    setError(false);

    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      try {
        const canvas = document.createElement('canvas');
        canvas.width = targetSize;
        canvas.height = targetSize;
        const ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.imageSmoothingEnabled = true;
          ctx.imageSmoothingQuality = 'high';
          ctx.drawImage(img, 0, 0, targetSize, targetSize);
          setResizedUrl(canvas.toDataURL('image/png'));
        }
      } catch {
        setResizedUrl(src);
      }
    };
    img.onerror = () => {
      setError(true);
    };
    img.src = src;
  }, [src, targetSize]);

  return { resizedUrl, error };
}

// ── Component ───────────────────────────────────────────────────────

type Step = 'username' | 'pin';

/** Staff login screen — two-step authentication flow with username entry followed by PIN pad input and shake animation on error. */
export default function StaffLoginScreen() {
  const { l10n } = useLocalization();
  const { login, loading: authLoading, error, clearError } = useAuth();
  const { settings: brandSettings, loading: brandLoading } = useBrand();
  // Convert local filesystem path to a Tauri-compatible asset URL.
  const logoUrl = useMemo(() => {
    if (brandLoading) return null;
    const path = brandSettings?.logo_path;
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return path; // fallback — may show broken image but won't crash
    }
  }, [brandLoading, brandSettings?.logo_path]);

  const { resizedUrl, error: primaryLogoError } = useResizedLogo(logoUrl, 256);
  const [fallbackSvgError, setFallbackSvgError] = useState(false);
  const { addToast } = useToast();
  const [step, setStep] = useState<Step>('username');
  const [username, setUsername] = useState('');
  const [usernameChecking, setUsernameChecking] = useState(false);
  const [pin, setPin] = useState<string[]>([]);
  const usernameInputRef = useRef<HTMLInputElement>(null);
  const pinWrapRef = useRef<HTMLDivElement>(null);
  const pinSubmitted = useRef(false);
  const [pinAttempts, setPinAttempts] = useState(0);
  const [lockedUntil, setLockedUntil] = useState<number | null>(null);
  const toastShownForError = useRef<string | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  // ── Shake card + toast + rate-limit on PIN error ──────────────

  useEffect(() => {
    if (!error || step !== 'pin' || error === toastShownForError.current) return;
    toastShownForError.current = error;

    const card = cardRef.current;
    if (card) {
      card.classList.add('staff-login-card--shake');
      setTimeout(() => card.classList.remove('staff-login-card--shake'), 350);
    }

    addToast({ type: 'error', message: error, duration: 5000 });
    setPin([]);

    // Track failed PIN attempts for client-side rate limiting
    setPinAttempts((prev) => {
      const next = prev + 1;
      if (next >= MAX_PIN_ATTEMPTS) {
        // Lock out for 30 seconds
        setLockedUntil(Date.now() + 30_000);
      }
      return next;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [error]);

  // ── Auto-unlock after lockout period ──────────────────────────

  useEffect(() => {
    if (lockedUntil === null) return;
    const remaining = lockedUntil - Date.now();
    if (remaining <= 0) {
      setLockedUntil(null);
      setPinAttempts(0);
      return;
    }
    const timer = setTimeout(() => {
      setLockedUntil(null);
      setPinAttempts(0);
    }, remaining);
    return () => clearTimeout(timer);
  }, [lockedUntil]);

  // ── Rate-limit display helpers ─────────────────────────────────

  const remainingAttempts = Math.max(0, MAX_PIN_ATTEMPTS - pinAttempts);
  const isLocked = lockedUntil !== null;
  const lockoutRemainingSec = lockedUntil !== null
    ? Math.max(0, Math.ceil((lockedUntil - Date.now()) / 1000))
    : 0;

  // ── Focus appropriate element when step changes ──────────────

  useEffect(() => {
    if (step === 'username') {
      usernameInputRef.current?.focus();
    } else if (step === 'pin') {
      pinWrapRef.current?.focus();
    }
  }, [step]);

  // ── Reset errors when step changes ──────────────────────────

  useEffect(() => {
    clearError();
  }, [step, clearError]);

  // ── PIN pad handlers ─────────────────────────────────────────

  const handlePinDigit = useCallback((digit: string) => {
    if (isLocked) return;
    setPin((prev) => {
      if (prev.length >= MAX_PIN_LENGTH) return prev;
      return [...prev, digit];
    });
  }, [isLocked]);

  const handlePinBackspace = useCallback(() => {
    if (isLocked) return;
    setPin((prev) => prev.slice(0, -1));
  }, [isLocked]);

  const handlePinClear = useCallback(() => {
    if (isLocked) return;
    setPin([]);
    pinSubmitted.current = false;
  }, [isLocked]);

  // ── Attempt login ────────────────────────────────────────────

  const attemptLogin = useCallback(() => {
    if (pin.length >= 1) {
      login(username.trim(), pin.join(''));
    }
  }, [pin, username, login]);

  // ── Back button ──────────────────────────────────────────────

  const goBack = useCallback(() => {
    setStep('username');
    setPin([]);
    pinSubmitted.current = false;
  }, []);

  // ── Hardware keyboard handler for PIN step ──────────────────

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (step !== 'pin') return;
      if (authLoading) return;

      if (e.key >= '0' && e.key <= '9') {
        e.preventDefault();
        handlePinDigit(e.key);
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        handlePinBackspace();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        handlePinClear();
        goBack();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (pin.length >= 1 && !pinSubmitted.current) attemptLogin();
      }
    },
    [step, authLoading, handlePinDigit, handlePinBackspace, handlePinClear, goBack, attemptLogin, pin.length],
  );

  // ── Username handlers ────────────────────────────────────────

  const handleUsernameSubmit = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = username.trim();
    if (!trimmed) return;

    setUsernameChecking(true);
    clearError();

    try {
      const result = await checkUsername({ username: trimmed });
      if (!result.found) {
        addToast({ type: 'error', message: l10n.getString('staff-login-error-not-found') });
        return;
      }
      if (!result.is_active) {
        addToast({ type: 'error', message: l10n.getString('staff-login-error-deactivated') });
        return;
      }
      setStep('pin');
    } catch {
      addToast({ type: 'error', message: l10n.getString('staff-login-error-connection') });
    } finally {
      setUsernameChecking(false);
    }
  }, [username, clearError, addToast, l10n]);

  const handleUsernameChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setUsername(e.target.value);
  }, []);

  const handleUsernameKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      setUsername('');
    }
  }, []);

  // ── Auto-submit when PIN reaches max length ──────────────────

  useEffect(() => {
    // Reset error gate when user starts re-entering a PIN after error
    if (pin.length === 1) {
      toastShownForError.current = null;
    }
    if (pin.length === MAX_PIN_LENGTH && !authLoading && !pinSubmitted.current) {
      pinSubmitted.current = true;
      attemptLogin();
    }
    if (pin.length < MAX_PIN_LENGTH) {
      pinSubmitted.current = false;
    }
  }, [pin, authLoading, attemptLogin]);

  // ── Focus on screen tap ──────────────────────────────────────

  const handleScreenClick = useCallback(() => {
    if (step === 'username') {
      usernameInputRef.current?.focus();
    } else if (step === 'pin') {
      pinWrapRef.current?.focus();
    }
  }, [step]);

  // ── Logo renderer ────────────────────────────────────────────

  const renderLogo = (small = false) => {
    const logoClass = `staff-login-logo${small ? ' staff-login-logo--small' : ''}`;
    const storeName = brandSettings?.store_name || '';

    // While brand settings are loading, show a skeleton placeholder
    // so the logo doesn't flash between different images.
    if (brandLoading) {
      return (
        <div className={logoClass}>
          <div className="staff-login-logo skeleton" />
        </div>
      );
    }

    return (
      <div className={logoClass}>
        {!primaryLogoError && resizedUrl ? (
          <img
            src={resizedUrl}
            alt={storeName || 'OZ-POS'}
            className="staff-login-logo-img"
          />
        ) : !fallbackSvgError ? (
          <img
            src="/branding/logo-mark.svg"
            alt={storeName || 'OZ-POS'}
            className="staff-login-logo-img"
            onError={() => setFallbackSvgError(true)}
          />
        ) : (
          <UserIcon />
        )}
      </div>
    );
  };

  // ── PIN dots ─────────────────────────────────────────────────

  const renderPinDots = () => (
    <div
      className="staff-login-pin-dots"
      aria-label={l10n.getString('staff-login-pin-aria', { length: pin.length, max: MAX_PIN_LENGTH })}
    >
      {Array.from({ length: MAX_PIN_LENGTH }, (_, i) => (
        <span
          key={i}
          className={`staff-login-pin-dot ${i < pin.length ? 'staff-login-pin-dot--filled' : ''}`}
          aria-hidden="true"
        />
      ))}
    </div>
  );

  // ── PIN pad ──────────────────────────────────────────────────

  const renderPinPad = () => {
    const keys = [
      ['7', '8', '9'],
      ['4', '5', '6'],
      ['1', '2', '3'],
    ];

    return (
      <div className="staff-login-pad" role="group" aria-label={l10n.getString('staff-login-keypad-aria')} aria-disabled={isLocked || undefined}>
        {keys.map((row) => (
          <div className="staff-login-pad-row" key={row[0]}>
            {row.map((digit) => (
              <Localized id="staff-login-digit-aria" attrs={{ 'aria-label': true }} vars={{ digit }} key={digit}>
                <button
                  type="button"
                  className="staff-login-pad-key"
                  onClick={() => handlePinDigit(digit)}                          aria-label={digit}                      disabled={authLoading || isLocked}
                    >
                      {digit}
                    </button>
                  </Localized>
                ))}
          </div>
        ))}
        <div className="staff-login-pad-row">
          <Localized id="staff-login-clear-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="staff-login-pad-key staff-login-pad-key--action"
              onClick={handlePinClear}
              aria-label="Clear"
              disabled={authLoading || pin.length === 0 || isLocked}
            >
              <Localized id="staff-login-clear">Clear</Localized>
            </button>
          </Localized>
          <Localized id="staff-login-digit-aria" attrs={{ 'aria-label': true }} vars={{ digit: '0' }}>
            <button
              type="button"
              className="staff-login-pad-key"
              onClick={() => handlePinDigit('0')}
              aria-label="0"
              disabled={authLoading || isLocked}
            >
              0
            </button>
          </Localized>
          <Localized id="staff-login-backspace-aria" attrs={{ 'aria-label': true }}>
            <button
              type="button"
              className="staff-login-pad-key staff-login-pad-key--action"
              onClick={handlePinBackspace}
              aria-label="Backspace"
              disabled={authLoading || pin.length === 0 || isLocked}
            >
              <BackspaceIcon />
            </button>
          </Localized>
        </div>
      </div>
    );
  };

  // ── Render ───────────────────────────────────────────────────

  // Focus management: clicking anywhere refocuses the active input
  const handleScreenKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === ' ') {
        e.preventDefault();
        handleScreenClick();
      }
    },
    [handleScreenClick],
  );

  return (
    /* eslint-disable-next-line jsx-a11y/no-static-element-interactions -- focus convenience, keyboard covered below */
    <div
      className="staff-login-screen"
      onClick={handleScreenClick}
      onKeyDown={handleScreenKeyDown}
      tabIndex={-1}
    >
      <div className={`staff-login-card ${step === 'pin' ? 'staff-login-card--pin' : ''}`} ref={cardRef}>
        {step === 'pin' && (
          <button
            type="button"
            className="staff-login-close-btn"
            onClick={goBack}
            aria-label={l10n.getString('staff-login-close-aria')}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" aria-hidden="true">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}

        {/* ── Top bar: logo (username) / store + dots (PIN) ── */}
        <div className="staff-login-top-bar">
          {step === 'username' && renderLogo()}
          {step === 'pin' && (
            <div className="staff-login-pin-top">
              {renderPinDots()}
            </div>
          )}
        </div>

        {/* ── Main area: form ────────────────────────────────── */}
        <div className="staff-login-main-area">
          {step === 'username' && (
            <form onSubmit={handleUsernameSubmit} className="staff-login-form">
              <div className="staff-login-input-wrap">
                <Localized id="staff-login-username-placeholder" attrs={{ placeholder: true }}>
                  <Localized id="staff-login-username-aria" attrs={{ 'aria-label': true }}>
                    <input
                      ref={usernameInputRef}
                      type="text"
                      id="staff-login-username"
                      name="username-off"
                      className="staff-login-input"
                      placeholder="Username"
                      value={username}
                      onChange={handleUsernameChange}
                      onKeyDown={handleUsernameKeyDown}
                      autoComplete="off"
                      autoCorrect="off"
                      spellCheck={false}
                      data-1p-ignore="true"
                      aria-label="Username"
                      disabled={authLoading}
                    />
                  </Localized>
                </Localized>
                <button
                  type="submit"
                  className="staff-login-submit-btn"
                  disabled={!username.trim() || usernameChecking}
                  aria-label={l10n.getString('staff-login-next-aria')}
                >
                  {usernameChecking ? (
                    <span className="staff-login-btn-spinner" />
                  ) : (
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" aria-hidden="true">
                      <line x1="5" y1="12" x2="19" y2="12" />
                      <polyline points="12 5 19 12 12 19" />
                    </svg>
                  )}
                </button>
              </div>
            </form>
          )}

          {step === 'pin' && (
            // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
            <div
              className="staff-login-pin-wrap"
              ref={pinWrapRef}
              tabIndex={-1}
              onKeyDown={handleKeyDown}
              role="application"
              aria-label={l10n.getString('staff-login-pin-section-aria')}
            >
              {renderPinPad()}
            </div>
          )}

          {error && step === 'username' && (
            <div className="staff-login-error" role="alert" aria-live="polite">
              <AlertIcon />
              {error}
            </div>
          )}

          {error && step === 'pin' && (
            <div className="staff-login-error" role="alert" aria-live="polite">
              <AlertIcon />
              {error}
              {pinAttempts >= RATE_LIMIT_WARN_AFTER && pinAttempts < MAX_PIN_ATTEMPTS && (
                <span className="staff-login-rate-limit">
                  {' '}{l10n.getString('staff-login-attempts-remaining', { count: String(remainingAttempts) })}
                </span>
              )}
              {isLocked && (
                <span className="staff-login-rate-limit staff-login-rate-limit--lockout">
                  {' '}{l10n.getString('staff-login-lockout', { seconds: String(lockoutRemainingSec) })}
                </span>
              )}
            </div>
          )}
        </div>

        {/* ── Bottom bar: step indicator (12%) ──────────────── */}
        <div className="staff-login-bottom-bar">
          <div
            className="staff-login-steps"
            role="status"
            aria-label={l10n.getString('staff-login-progress-aria')}
          >
            <span className={`staff-login-step-dot ${step === 'username' ? 'staff-login-step-dot--active' : 'staff-login-step-dot--done'}`} />
            <span className="staff-login-step-line" />
            <span className={`staff-login-step-dot ${step === 'pin' ? 'staff-login-step-dot--active' : ''}`} />
          </div>
        </div>
      </div>

      {/* ── Footer: version + copyright ────────────────────── */}
      <div className="staff-login-footer">
        <span className="staff-login-footer-version">OZ-POS Enterprise v0.0.12</span>
        <Localized id="staff-login-copyright">
          <span className="staff-login-footer-copyright">&copy; 2026 OZ-POS. All rights reserved.</span>
        </Localized>
      </div>
    </div>
  );
}
