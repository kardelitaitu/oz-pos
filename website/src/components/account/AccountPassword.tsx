import { t } from '../../i18n';
import { isStrongPassword, passwordsMatch } from '../../lib/passwordPolicy';
import PasswordField from '../PasswordField';
import PasswordStrength from '../PasswordStrength';

/**
 * Optional password management — set or change the login credential while
 * signed in. Owns its form state and calls `onSave(pw)` when valid; the
 * parent performs the API call and reports success/failure via `msg`.
 */
export type PasswordMsg = 'idle' | 'saved' | 'error';

interface Props {
  locale: string;
  pw: string;
  pwConfirm: string;
  msg: PasswordMsg;
  saving: boolean;
  onPwChange: (v: string) => void;
  onPwConfirmChange: (v: string) => void;
  onSave: (pw: string) => void;
}

export default function AccountPassword({ locale, pw, pwConfirm, msg, saving, onPwChange, onPwConfirmChange, onSave }: Props) {
  return (
    <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.password')}>
      <h2 className="text-lg font-semibold">{t(locale, 'account.password')}</h2>
      <p className="mt-1 text-sm text-muted">{t(locale, 'account.passwordHelp')}</p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSave(pw);
        }}
        className="mt-4 space-y-3"
      >
        <PasswordField
          locale={locale}
          id="account-password"
          label={t(locale, 'account.passwordPlaceholder')}
          value={pw}
          onChange={onPwChange}
          autoComplete="new-password"
          placeholder={t(locale, 'account.passwordPlaceholder')}
          showConfirm
          confirmValue={pwConfirm}
          onConfirmChange={onPwConfirmChange}
        />
        {msg === 'saved' && (
          <p className="text-sm text-success" role="status">{t(locale, 'account.passwordSaved')}</p>
        )}
        {msg === 'error' && (
          <p className="text-sm text-danger" role="alert">{t(locale, 'account.passwordError')}</p>
        )}
        <PasswordStrength locale={locale} password={pw} />
        <button
          type="submit"
          disabled={saving || !isStrongPassword(pw) || !passwordsMatch(pw, pwConfirm)}
          className="rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-60"
        >
          {saving ? '…' : t(locale, 'account.passwordSave')}
        </button>
      </form>
    </section>
  );
}
