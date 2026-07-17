import { useState, useCallback, useEffect, useRef } from "react";
import { useAuth } from "@/contexts/AuthContext";
import { useWorkspace } from "@/contexts/WorkspaceContext";
import { staffLogin } from "@/api/staff";
import { Localized } from "@/frontend/shared/Localized";
import { useLocalization } from "@fluent/react";
import "./FastPINOverlay.css";

// ── SVG icons ───────────────────────────────────────────────────────

function BackspaceIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M21 4H8l-7 8 7 8h13a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z" />
      <line x1="18" y1="9" x2="12" y2="15" />
      <line x1="12" y1="9" x2="18" y2="15" />
    </svg>
  );
}

function AlertIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      width="16"
      height="16"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="15" y1="9" x2="9" y2="15" />
      <line x1="9" y1="9" x2="15" y2="15" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      width="18"
      height="18"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  );
}

function UserIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
      <circle cx="12" cy="7" r="4" />
    </svg>
  );
}

// ── Constants ───────────────────────────────────────────────────────

const MAX_PIN_LENGTH = 4;

// ── Types ───────────────────────────────────────────────────────────

export interface FastPINOverlayProps {
  /** Whether the overlay is visible. */
  open: boolean;
  /** Called when the overlay should close (cancel or Escape). */
  onClose: () => void;
}

// ── Component ───────────────────────────────────────────────────────

type Step = "username" | "pin";

/**
 * Quick Staff PIN Pad overlay for shared touchscreen operator switching.
 *
 * ADR #6: Allows staff to hot-swap operators on a shared POS terminal
 * without logging out and back in. Verifies the new operator's PIN,
 * creates a new session token with the same scope (store, instance,
 * terminal) but the new user's identity, and preserves the active
 * workspace.
 *
 * Flow:
 * 1. User enters username
 * 2. User enters PIN on a numeric keypad
 * 3. On verification: swaps AuthContext session + WorkspaceContext token
 * 4. Overlay dismisses
 */
export default function FastPINOverlay({ open, onClose }: FastPINOverlayProps) {
  const { l10n } = useLocalization();
  const { session, swapSession } = useAuth();
  const { swapSessionToken } = useWorkspace();

  const [step, setStep] = useState<Step>("username");
  const [username, setUsername] = useState("");
  const [pin, setPin] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exiting, setExiting] = useState(false);
  const usernameInputRef = useRef<HTMLInputElement>(null);
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pinSubmitted = useRef(false);

  // ── Reset state on open ──────────────────────────────────────

  useEffect(() => {
    if (open) {
      setStep("username");
      setUsername("");
      setPin([]);
      setError(null);
      setLoading(false);
      pinSubmitted.current = false;
    }
  }, [open]);

  // ── Focus username input on step change ──────────────────────

  useEffect(() => {
    if (open && step === "username") {
      // Small delay so the DOM has rendered
      const timer = setTimeout(() => usernameInputRef.current?.focus(), 50);
      return () => clearTimeout(timer);
    }
  }, [open, step]);

  // ── Cleanup exit timer on unmount ─────────────────────────-

  useEffect(() => {
    return () => {
      if (exitTimerRef.current !== null) {
        clearTimeout(exitTimerRef.current);
        exitTimerRef.current = null;
      }
    };
  }, []);

  // ── Animated close (sets exiting, delays unmount) ───────────

  const handleClose = useCallback(() => {
    if (loading) return;
    setExiting(true);
    exitTimerRef.current = setTimeout(() => {
      setExiting(false);
      exitTimerRef.current = null;
      onClose();
    }, 200);
  }, [onClose, loading]);

  // ── Escape key closes ───────────────────────────────────────

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !exiting) {
        setExiting(true);
        exitTimerRef.current = setTimeout(() => {
          setExiting(false);
          exitTimerRef.current = null;
          onClose();
        }, 200);
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open, onClose, exiting]);

  // ── Verify PIN and swap session ─────────────────────────────

  const attemptVerify = useCallback(async () => {
    if (pin.length === 0) return;
    setLoading(true);
    setError(null);
    try {
      // Step 1: Verify the new operator's credentials.
      const result = await staffLogin({
        username: username.trim(),
        pin: pin.join(""),
      });
      const newSession = result.session;

      // Step 2: Swap the AuthContext session (no workspace reset).
      swapSession(newSession);

      // Step 3: Create a new session token with the new user's identity
      // but the same scope (store, instance, terminal).
      await swapSessionToken(newSession.user_id, newSession.role_id);

      // Step 4: Dismiss the overlay (after successful verify — snap close intentional).
      onClose();
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "PIN verification failed";
      setError(message);
      setPin([]);
    } finally {
      setLoading(false);
    }
  }, [pin, username, swapSession, swapSessionToken, onClose]);

  // ── Auto-submit when PIN reaches max length ─────────────────

  useEffect(() => {
    if (pin.length === MAX_PIN_LENGTH && !loading && !pinSubmitted.current) {
      pinSubmitted.current = true;
      attemptVerify();
    }
    if (pin.length < MAX_PIN_LENGTH) {
      pinSubmitted.current = false;
    }
  }, [pin, loading, attemptVerify]);

  // ── PIN pad handlers ────────────────────────────────────────

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

  // ── Back button ─────────────────────────────────────────────

  const goBack = useCallback(() => {
    setStep("username");
    setPin([]);
    pinSubmitted.current = false;
  }, []);

  // ── Hardware keyboard support ───────────────────────────────

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (step !== "pin" || loading) return;

      if (e.key >= "0" && e.key <= "9") {
        e.preventDefault();
        handlePinDigit(e.key);
      } else if (e.key === "Backspace") {
        e.preventDefault();
        handlePinBackspace();
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (pin.length >= 1 && !pinSubmitted.current) attemptVerify();
      }
    },
    [
      step,
      loading,
      handlePinDigit,
      handlePinBackspace,
      attemptVerify,
      pin.length,
    ],
  );

  // ── Username handlers ───────────────────────────────────────

  const handleUsernameSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (username.trim()) {
        setStep("pin");
      }
    },
    [username],
  );

  // ── Render helpers ────────────────────────────────────────────────

  const renderPinDots = () => (
    <div
      className="fastpin-pin-dots"
      aria-label={l10n.getString("staff-login-pin-aria", {
        length: pin.length,
        max: MAX_PIN_LENGTH,
      })}
    >
      {Array.from({ length: MAX_PIN_LENGTH }, (_, i) => (
        <span
          key={i}
          className={`fastpin-pin-dot ${i < pin.length ? "fastpin-pin-dot--filled" : ""}`}
          aria-hidden="true"
        />
      ))}
    </div>
  );

  const renderPinPad = () => {
    const keys = [
      ["7", "8", "9"],
      ["4", "5", "6"],
      ["1", "2", "3"],
    ];

    return (
      <div
        className="fastpin-pad"
        role="group"
        aria-label={l10n.getString("staff-login-keypad-aria")}
      >
        {keys.map((row) => (
          <div className="fastpin-pad-row" key={row[0]}>
            {row.map((digit) => (
              <Localized
                id="staff-login-digit-aria"
                attrs={{ "aria-label": true }}
                vars={{ digit }}
                key={digit}
              >
                <button
                  type="button"
                  className="fastpin-pad-key"
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
        <div className="fastpin-pad-row">
          <Localized id="staff-login-clear-aria" attrs={{ "aria-label": true }}>
            <button
              type="button"
              className="fastpin-pad-key fastpin-pad-key--action"
              onClick={handlePinClear}
              aria-label="Clear"
              disabled={loading || pin.length === 0}
            >
              <Localized id="staff-login-clear">Clear</Localized>
            </button>
          </Localized>
          <Localized
            id="staff-login-digit-aria"
            attrs={{ "aria-label": true }}
            vars={{ digit: "0" }}
          >
            <button
              type="button"
              className="fastpin-pad-key"
              onClick={() => handlePinDigit("0")}
              aria-label="0"
              disabled={loading}
            >
              0
            </button>
          </Localized>
          <Localized
            id="staff-login-backspace-aria"
            attrs={{ "aria-label": true }}
          >
            <button
              type="button"
              className="fastpin-pad-key fastpin-pad-key--action"
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

  // ── Render ────────────────────────────────────────────────────────

  if (!open && !exiting) return null;

  // Escape key is handled via document-level keydown listener
  /* eslint-disable jsx-a11y/no-static-element-interactions, jsx-a11y/click-events-have-key-events */
  return (
    <div
      className={`fastpin-overlay${exiting ? ' fastpin-overlay--exiting' : ''}`}
      onClick={(e) => {
        if (e.target === e.currentTarget && !loading) handleClose();
      }}
    >
      {/* eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions */}
      <div
        className={`fastpin-card${exiting ? ' fastpin-card--exiting' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-label={l10n.getString("staff-login-pin-section-aria")}
        onKeyDown={handleKeyDown}
      >
        {/* ── Close button ──────────────────────────────── */}
        <button
          type="button"
          className="fastpin-close-btn"
          onClick={handleClose}
          aria-label={l10n.getString("modal-close-aria")}
          disabled={loading}
        >
          <CloseIcon />
        </button>

        {/* ── Username step ─────────────────────────────── */}
        {step === "username" && (
          <div className="fastpin-step-content" key="username">
            <div className="fastpin-icon-wrap">
              <UserIcon />
            </div>

            <Localized id="staff-login-step-username">
              <p className="fastpin-step-label">Enter your username</p>
            </Localized>

            <form onSubmit={handleUsernameSubmit} className="fastpin-form">
              <Localized
                id="staff-login-username-placeholder"
                attrs={{ placeholder: true }}
              >
                <Localized
                  id="staff-login-username-aria"
                  attrs={{ "aria-label": true }}
                >
                  <input
                    ref={usernameInputRef}
                    type="text"
                    className="fastpin-input"
                    id="fastpin-username"
                    name="fastpin-username"
                    placeholder="Username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    autoComplete="off"
                    aria-label="Username"
                    disabled={loading}
                  />
                </Localized>
              </Localized>
              <Localized id="staff-login-next">
                <button
                  type="submit"
                  className="fastpin-submit-btn"
                  disabled={!username.trim() || loading}
                >
                  {loading ? (
                    <>
                      <span className="fastpin-btn-spinner" />
                      <Localized id="staff-login-submitting">
                        <span>Verifying…</span>
                      </Localized>
                    </>
                  ) : (
                    "Next"
                  )}
                </button>
              </Localized>
            </form>
          </div>
        )}

        {/* ── PIN step ──────────────────────────────────── */}
        {step === "pin" && (
          <div className="fastpin-step-content" key="pin">
            <div className="fastpin-icon-wrap fastpin-icon-wrap--small">
              <UserIcon />
            </div>

            {session?.display_name && (
              <p className="fastpin-active-user">
                <Localized
                  id="fastpin-active-user"
                  vars={{ name: session.display_name }}
                >
                  <span>Active: {session.display_name}</span>
                </Localized>
              </p>
            )}

            <Localized id="fastpin-enter-pin">
              <p className="fastpin-step-label">
                Enter PIN for {username.trim()}
              </p>
            </Localized>

            {renderPinDots()}
            {renderPinPad()}

            {/* Submit button for PINs shorter than max */}
            <button
              type="button"
              className="fastpin-submit-btn fastpin-pin-submit"
              onClick={attemptVerify}
              disabled={pin.length === 0 || loading}
            >
              {loading ? (
                <>
                  <span className="fastpin-btn-spinner" />
                  <Localized id="staff-login-submitting">
                    <span>Verifying…</span>
                  </Localized>
                </>
              ) : (
                l10n.getString("staff-login-submit")
              )}
            </button>

            {/* Back button */}
            <Localized id="staff-login-back">
              <button
                type="button"
                className="fastpin-back-btn"
                onClick={goBack}
                disabled={loading}
              >
                &larr; Back
              </button>
            </Localized>
          </div>
        )}

        {/* ── Error ──────────────────────────────────────── */}
        {error && (
          <div className="fastpin-error" role="alert">
            <AlertIcon />
            {error}
          </div>
        )}

        {/* ── Cancel footer ──────────────────────────────── */}
        <div className="fastpin-footer">
          <button
            type="button"
            className="fastpin-cancel-btn"
            onClick={handleClose}
            disabled={loading}
          >
            <Localized id="cancel">Cancel</Localized>
          </button>
        </div>
      </div>
    </div>
  );
}
