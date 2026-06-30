import { useState, useCallback, useRef, useEffect } from 'react';
import { useAuth } from '@/contexts/AuthContext';
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

// ── Constants ───────────────────────────────────────────────────────

const MAX_PIN_LENGTH = 6;

// ── Component ───────────────────────────────────────────────────────

type Step = 'username' | 'pin';

export default function StaffLoginScreen() {
  const { login, loading, error, clearError } = useAuth();

  const [step, setStep] = useState<Step>('username');
  const [username, setUsername] = useState('');
  const [pin, setPin] = useState<string[]>([]);
  const usernameInputRef = useRef<HTMLInputElement>(null);

  const pinSectionRef = useRef<HTMLDivElement>(null);

  // Focus appropriate element when step changes.
  useEffect(() => {
    if (step === 'username') {
      usernameInputRef.current?.focus();
    } else if (step === 'pin') {
      pinSectionRef.current?.focus();
    }
  }, [step]);

  // Reset error when step changes.
  useEffect(() => {
    clearError();
  }, [step, clearError]);

  // ── PIN pad handlers ───────────────────────────────────────────

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
  }, []);

  // Auto-submit when PIN reaches max length.
  const attemptLogin = useCallback(() => {
    if (pin.length >= 1) {
      login(username.trim(), pin.join(''));
    }
  }, [pin, username, login]);

  // ── Back button ────────────────────────────────────────────────

  const goBack = useCallback(() => {
    setStep('username');
    setPin([]);
  }, []);

  // ── Hardware keyboard handler for PIN step ────────────────────

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

  // ── Username handlers ──────────────────────────────────────────

  const handleUsernameSubmit = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (username.trim()) {
      setStep('pin');
    }
  }, [username]);

  const handleUsernameChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setUsername(e.target.value);
  }, []);

  useEffect(() => {
    if (pin.length === MAX_PIN_LENGTH && !loading) {
      attemptLogin();
    }
  }, [pin, loading, attemptLogin]);

  // ── PIN entry visual ───────────────────────────────────────────

  const renderPinDots = (length: number) => {
    return (
      <div className="staff-login-pin-dots" aria-label={`PIN entry: ${length} of ${MAX_PIN_LENGTH} digits`}>
        {Array.from({ length: MAX_PIN_LENGTH }, (_, i) => (
          <span
            key={i}
            className={`staff-login-pin-dot ${i < length ? 'staff-login-pin-dot--filled' : ''}`}
            aria-hidden="true"
          />
        ))}
      </div>
    );
  };

  // ── PIN pad grid ─────────────────────────────────────────────

  const renderPinPad = () => {
    const keys = [
      ['1', '2', '3'],
      ['4', '5', '6'],
      ['7', '8', '9'],
    ];

    return (
      <div className="staff-login-pad" role="group" aria-label="Numeric keypad">
        {keys.map((row) => (
          <div className="staff-login-pad-row" key={row[0]}>
            {row.map((digit) => (
              <button
                key={digit}
                type="button"
                className="staff-login-pad-key"
                onClick={() => handlePinDigit(digit)}
                aria-label={digit}
                disabled={loading}
              >
                {digit}
              </button>
            ))}
          </div>
        ))}
        <div className="staff-login-pad-row">
          <button
            type="button"
            className="staff-login-pad-key staff-login-pad-key--clear"
            onClick={handlePinClear}
            aria-label="Clear"
            disabled={loading || pin.length === 0}
          >
            Clear
          </button>
          <button
            type="button"
            className="staff-login-pad-key"
            onClick={() => handlePinDigit('0')}
            aria-label="0"
            disabled={loading}
          >
            0
          </button>
          <button
            type="button"
            className="staff-login-pad-key staff-login-pad-key--backspace"
            onClick={handlePinBackspace}
            aria-label="Backspace"
            disabled={loading || pin.length === 0}
          >
            <BackspaceIcon />
          </button>
        </div>
      </div>
    );
  };

  // ── Step label ────────────────────────────────────────────────

  const stepLabel = step === 'username' ? 'Enter your username' : 'Enter your PIN';

  return (
    <div className="staff-login-screen">
      <div className="staff-login-card">
        {/* Logo */}
        <div className="staff-login-logo">
          <UserIcon />
        </div>
        <h1 className="staff-login-title">OZ-POS</h1>
        <p className="staff-login-subtitle">Staff Login</p>

        {/* Step indicator */}
        <div className="staff-login-steps" role="progressbar" aria-valuenow={step === 'username' ? 1 : 2} aria-valuemin={1} aria-valuemax={2} aria-label="Login progress">
          <span className={`staff-login-step-dot ${step === 'username' ? 'staff-login-step-dot--active' : 'staff-login-step-dot--done'}`} />
          <span className="staff-login-step-line" />
          <span className={`staff-login-step-dot ${step === 'pin' ? 'staff-login-step-dot--active' : ''}`} />
        </div>

        <p className="staff-login-step-label">{stepLabel}</p>

        {/* Username step */}
        {step === 'username' && (
          <form onSubmit={handleUsernameSubmit} className="staff-login-form">
            <div className="staff-login-input-wrap">
              <input
                ref={usernameInputRef}
                type="text"
                className="staff-login-input"
                placeholder="Username"
                value={username}
                onChange={handleUsernameChange}
                autoComplete="username"
                aria-label="Username"
                disabled={loading}
              />
            </div>
            <button
              type="submit"
              className="staff-login-submit-btn"
              disabled={!username.trim() || loading}
            >
              Next
            </button>
          </form>
        )}

        {/* PIN step */}
        {step === 'pin' && (
          // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions
          <div
            className="staff-login-pin-section"
            ref={pinSectionRef}
            tabIndex={-1}
            onKeyDown={handleKeyDown}
            role="application"
            aria-label="PIN entry — type digits on your keyboard or use the on-screen keypad"
          >
            {renderPinDots(pin.length)}
            {renderPinPad()}

            {/* Submit button for PINs shorter than max length */}
            <button
              type="button"
              className="staff-login-submit-btn staff-login-pin-submit"
              onClick={attemptLogin}
              disabled={pin.length === 0 || loading}
            >
              {loading ? 'Logging in…' : 'Log in'}
            </button>
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="staff-login-error" role="alert">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16" aria-hidden="true">
              <circle cx="12" cy="12" r="10" />
              <line x1="15" y1="9" x2="9" y2="15" />
              <line x1="9" y1="9" x2="15" y2="15" />
            </svg>
            {error}
          </div>
        )}

        {/* Loading */}
        {loading && (
          <div className="staff-login-loading" role="status">
            <div className="staff-login-spinner" />
            <span>Verifying…</span>
          </div>
        )}

        {/* Back button (except on username step) */}
        {step !== 'username' && (
          <button
            type="button"
            className="staff-login-back-link"
            onClick={goBack}
            disabled={loading}
          >
            &larr; Back
          </button>
        )}
      </div>
    </div>
  );
}
