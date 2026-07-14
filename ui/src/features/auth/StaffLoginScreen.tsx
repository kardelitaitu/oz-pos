import { useState, useCallback, useRef, useEffect } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import { useOptionalBrand } from '@/contexts/BrandContext';
import { Localized } from '@/frontend/shared/Localized';
import { useLocalization } from '@fluent/react';
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
  const { login, loading, error, clearError } = useAuth();
  const brandSettings = useOptionalBrand();
  const rawLogoPath = brandSettings?.logo_path || '/256x256.png';
  const { resizedUrl, error: primaryLogoError } = useResizedLogo(rawLogoPath, 256);
  const [fallbackSvgError, setFallbackSvgError] = useState(false);
  const [step, setStep] = useState<Step>('username');
  const [username, setUsername] = useState('');
  const [pin, setPin] = useState<string[]>([]);
  const usernameInputRef = useRef<HTMLInputElement>(null);
  const pinWrapRef = useRef<HTMLDivElement>(null);
  const pinSubmitted = useRef(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // ── Shake card on error ──────────────────────────────────────

  const prevErrorRef = useRef<string | null>(null);
  useEffect(() => {
    if (error && error !== prevErrorRef.current) {
      const card = cardRef.current;
      if (card) {
        card.classList.add('staff-login-card--shake');
        const timer = setTimeout(() => card.classList.remove('staff-login-card--shake'), 350);
        return () => clearTimeout(timer);
      }
    }
    prevErrorRef.current = error;
  }, [error]);

  // ── Focus appropriate element when step changes ──────────────

  useEffect(() => {
    if (step === 'username') {
      usernameInputRef.current?.focus();
    } else if (step === 'pin') {
      pinWrapRef.current?.focus();
    }
  }, [step]);

  // ── Reset error when step changes ────────────────────────────

  useEffect(() => {
    clearError();
  }, [step, clearError]);

  // ── PIN pad handlers ─────────────────────────────────────────

  const handlePinDigit = useCallback((digit: string) => {
    setPin((prev) => {
      if (prev.length >= MAX_PIN_LENGTH) return prev;
      return [...prev, digit];
    });
  }, []);

  const handlePinBackspace = useCallback(() => {
    setPin((prev) => prev.slice(0, -1));
  }, []);

  const handlePinClear = useCallback(() => {
    setPin([]);
    pinSubmitted.current = false;
  }, []);

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
      if (loading) return;

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
        if (pin.length >= 1) attemptLogin();
      }
    },
    [step, loading, handlePinDigit, handlePinBackspace, handlePinClear, goBack, attemptLogin, pin.length],
  );

  // ── Username handlers ────────────────────────────────────────

  const handleUsernameSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (username.trim()) {
      setStep('pin');
    }
  }, [username]);

  const handleUsernameChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setUsername(e.target.value);
  }, []);

  // ── Auto-submit when PIN reaches max length ──────────────────

  useEffect(() => {
    if (pin.length === MAX_PIN_LENGTH && !loading && !pinSubmitted.current) {
      pinSubmitted.current = true;
      attemptLogin();
    }
    if (pin.length < MAX_PIN_LENGTH) {
      pinSubmitted.current = false;
    }
  }, [pin, loading, attemptLogin]);

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
      <div className="staff-login-pad" role="group" aria-label={l10n.getString('staff-login-keypad-aria')}>
        {keys.map((row) => (
          <div className="staff-login-pad-row" key={row[0]}>
            {row.map((digit) => (
              <Localized id="staff-login-digit-aria" attrs={{ 'aria-label': true }} vars={{ digit }} key={digit}>
                <button
                  type="button"
                  className="staff-login-pad-key"
                  onClick={() => handlePinDigit(digit)}
                  aria-label={digit}
                  disabled={loading}
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
              disabled={loading || pin.length === 0}
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
              disabled={loading}
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
              disabled={loading || pin.length === 0}
            >
              <BackspaceIcon />
            </button>
          </Localized>
        </div>
      </div>
    );
  };

  // ── Render ───────────────────────────────────────────────────

  const storeName = brandSettings?.store_name || '';

  return (
    <div className="staff-login-screen" onClick={handleScreenClick} role="presentation">
      <div className="staff-login-card" ref={cardRef}>
        {/* ── Username step ─────────────────────────────────── */}
        {step === 'username' && (
          <div className="staff-login-step-content" key="username">
            {renderLogo()}

            <Localized id="staff-login-step-username">
              <p className="staff-login-step-label">Enter your username</p>
            </Localized>

            <form onSubmit={handleUsernameSubmit} className="staff-login-form">
              <div className="staff-login-input-wrap">
                <Localized id="staff-login-username-placeholder" attrs={{ placeholder: true }}>
                  <Localized id="staff-login-username-aria" attrs={{ 'aria-label': true }}>
                    <input
                      ref={usernameInputRef}
                      type="text"
                      id="staff-login-username"
                      name="username"
                      className="staff-login-input"
                      placeholder="Username"
                      value={username}
                      onChange={handleUsernameChange}
                      autoComplete="username"
                      aria-label="Username"
                      disabled={loading}
                    />
                  </Localized>
                </Localized>
              </div>
              <Localized id="staff-login-next">
                <button
                  type="submit"
                  className="staff-login-submit-btn"
                  disabled={!username.trim() || loading}
                >
                  {loading ? (
                    <>
                      <span className="staff-login-btn-spinner" />
                      <Localized id="staff-login-submitting"><span>Verifying…</span></Localized>
                    </>
                  ) : (
                    'Next'
                  )}
                </button>
              </Localized>
            </form>
          </div>
        )}

        {/* ── PIN step ──────────────────────────────────────── */}
        {step === 'pin' && (
          <div className="staff-login-step-content" key="pin">
            {renderLogo(true)}
            {storeName && <p className="staff-login-store-name">{storeName}</p>}

            {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
            <div
              className="staff-login-pin-wrap"
              ref={pinWrapRef}
              tabIndex={-1}
              onKeyDown={handleKeyDown}
              role="application"
              aria-label={l10n.getString('staff-login-pin-section-aria')}
            >
              {renderPinDots()}
              {renderPinPad()}

              {/* Submit button for PINs shorter than max length */}
              <button
                type="button"
                className="staff-login-submit-btn staff-login-pin-submit"
                onClick={attemptLogin}
                disabled={pin.length === 0 || loading}
              >
                {loading ? (
                  <>
                    <span className="staff-login-btn-spinner" />
                    <Localized id="staff-login-submitting">
                      <span>Verifying…</span>
                    </Localized>
                  </>
                ) : (
                  l10n.getString('staff-login-submit')
                )}
              </button>
            </div>

            {/* Back button */}
            <Localized id="staff-login-back">
              <button
                type="button"
                className="staff-login-back-btn"
                onClick={goBack}
                disabled={loading}
              >
                &larr; Back
              </button>
            </Localized>
          </div>
        )}

        {/* ── Error ──────────────────────────────────────────── */}
        {error && (
          <div className="staff-login-error" role="alert">
            <AlertIcon />
            {error}
          </div>
        )}

        {/* ── Step indicator ─────────────────────────────────── */}
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

      {/* ── Footer: version + copyright ────────────────────── */}
      <div className="staff-login-footer">
        <span className="staff-login-footer-version">OZ-POS Enterprise v0.0.7</span>
        <Localized id="staff-login-copyright">
          <span className="staff-login-footer-copyright">&copy; 2026 OZ-POS. All rights reserved.</span>
        </Localized>
      </div>
    </div>
  );
}
