import { t } from '../i18n';

/**
 * Password strength meter (mirrors the server policy in web_password.go):
 * at least 8 characters and at least 3 of the 4 character classes
 * (lowercase / uppercase / digit / symbol). Exported for reuse in
 * SignupForm, AuthForm (forgot-password), and AccountView (change).
 *
 * The meter shows 4 segments — one per class — lit in the strength color;
 * the label reads Too short / Weak / Fair / Good / Strong. isStrong is
 * the exact server gate, so the submit button can be disabled on the same
 * rule the server enforces.
 */

const CLASS_RE: RegExp[] = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^A-Za-z0-9]/];

/** How many of the 4 classes the password satisfies (0–4). */
export function passwordClassCount(password: string): number {
  return CLASS_RE.reduce((n, re) => (re.test(password) ? n + 1 : n), 0);
}

/** Server-mirroring gate: ≥8 chars and ≥3 classes. */
export function isStrongPassword(password: string): boolean {
  return password.length >= 8 && passwordClassCount(password) >= 3;
}

interface Props {
  locale: string;
  password: string;
}

export default function PasswordStrength({ locale, password }: Props) {
  const classes = passwordClassCount(password);
  const minLenOk = password.length >= 8;

  let labelKey = 'password.strengthTooShort';
  let color = 'var(--callout-danger)';
  if (minLenOk) {
    if (classes < 3) {
      labelKey = 'password.strengthWeak';
    } else if (classes === 3) {
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
