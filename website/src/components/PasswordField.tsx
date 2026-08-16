import { useState } from 'react';
import { t } from '../i18n';

/**
 * Password input shared by every auth form (signup, login, forgot-password
 * reset, dashboard change). Provides:
 *
 *   1. A show/hide toggle (👁 / 🙈, aria-labeled) — the eye icon swaps the
 *      input between text and password, so users can verify what they
 *      typed before submitting.
 *   2. An optional confirm field (showConfirm) — the double-entry guard.
 *      Both inputs share the same visibility state, and a "Passwords
 *      don't match" hint appears as soon as the confirm differs from the
 *      password. Parents gate their submit on passwordsMatch() from
 *      ../lib/passwordPolicy, and the server enforces the same rule via
 *      password_confirm (web_password.go).
 *
 * The label and placeholders come from props so callers can reuse generic
 * password.* i18n keys or supply form-specific ones.
 */
const inputClass =
  'w-full rounded-md border border-ink/10 bg-primary px-3 py-2 text-sm text-ink outline-none transition focus:border-accent';

interface Props {
  locale: string;
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  autoComplete: string;
  placeholder?: string;
  /** Show the second "confirm password" input below the first. */
  showConfirm?: boolean;
  confirmValue?: string;
  onConfirmChange?: (value: string) => void;
  confirmLabel?: string;
  confirmPlaceholder?: string;
}

export default function PasswordField({
  locale,
  id,
  label,
  value,
  onChange,
  autoComplete,
  placeholder,
  showConfirm = false,
  confirmValue = '',
  onConfirmChange,
  confirmLabel,
  confirmPlaceholder,
}: Props) {
  const [visible, setVisible] = useState(false);
  const mismatch = showConfirm && confirmValue !== '' && confirmValue !== value;

  return (
    <>
      <label className="block">
        <span className="mb-1 block text-sm text-muted">{label}</span>
        <span className="relative block">
          <input
            id={id}
            type={visible ? 'text' : 'password'}
            required
            autoComplete={autoComplete}
            minLength={8}
            maxLength={72}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={placeholder}
            className={`${inputClass} pr-10`}
          />
          <button
            type="button"
            aria-label={visible ? t(locale, 'password.hide') : t(locale, 'password.show')}
            onClick={() => setVisible((v) => !v)}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-sm text-muted transition hover:text-ink"
          >
            {visible ? '🙈' : '👁'}
          </button>
        </span>
      </label>
      {showConfirm && (
        <label className="block">
          <span className="mb-1 block text-sm text-muted">
            {confirmLabel ?? t(locale, 'password.confirmLabel')}
          </span>
          <input
            id={`${id}-confirm`}
            type={visible ? 'text' : 'password'}
            required
            autoComplete={autoComplete}
            minLength={8}
            maxLength={72}
            value={confirmValue}
            onChange={(e) => onConfirmChange?.(e.target.value)}
            placeholder={confirmPlaceholder ?? t(locale, 'password.confirmPlaceholder')}
            className={inputClass}
          />
        </label>
      )}
      {mismatch && (
        <p className="text-sm text-link" role="alert">
          {t(locale, 'password.mismatch')}
        </p>
      )}
    </>
  );
}
