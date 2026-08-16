import { t } from '../i18n';
import {
  passwordByteLength,
  passwordClassCount,
  passwordMaxBytes,
  passwordMinClasses,
  passwordMinLen,
  passwordRuneCount,
} from '../lib/passwordPolicy';

/**
 * Password strength meter — the policy itself lives in
 * ../lib/passwordPolicy.ts (the single client-side source of truth,
 * mirrored from the server's web_password.go and pinned to the shared
 * scripts/password-policy-cases.json fixture by both test suites). This
 * component only renders it.
 *
 * The meter shows 4 segments — one per class — lit in the strength color;
 * the label reads Too short / Weak / Fair / Good / Strong. isStrong is
 * the exact server gate, so the submit button can be disabled on the same
 * rule the server enforces.
 */

interface Props {
  locale: string;
  password: string;
}

export default function PasswordStrength({ locale, password }: Props) {
  const classes = passwordClassCount(password);
  const minLenOk =
    passwordByteLength(password) >= passwordMinLen &&
    passwordByteLength(password) <= passwordMaxBytes &&
    passwordRuneCount(password) >= passwordMinLen;

  let labelKey = 'password.strengthTooShort';
  let color = 'var(--callout-danger)';
  if (minLenOk) {
    if (classes < passwordMinClasses) {
      labelKey = 'password.strengthWeak';
    } else if (classes === passwordMinClasses) {
      labelKey = 'password.strengthGood';
      color = 'var(--callout-tip)';
    } else {
      labelKey = 'password.strengthStrong';
      color = 'var(--color-accent)';
    }
  }

  return (
    <div className="space-y-1.5">
      <div
        className="grid grid-cols-4 gap-1"
        role="meter"
        aria-label={t(locale, 'password.meterLabel')}
        aria-valuemin={0}
        aria-valuemax={4}
        aria-valuenow={classes}
      >
        {[0, 1, 2, 3].map((i) => (
          <span
            key={i}
            className="h-1.5 rounded-full transition-colors"
            style={{
              backgroundColor: i < classes ? color : 'color-mix(in srgb, var(--color-ink) 12%, transparent)',
            }}
          />
        ))}
      </div>
      <p className="text-xs text-muted">
        {t(locale, labelKey)}
        {!minLenOk && <span> — {t(locale, 'password.minLength')}</span>}
      </p>
      <p className="text-xs text-muted">{t(locale, 'password.hint')}</p>
    </div>
  );
}
