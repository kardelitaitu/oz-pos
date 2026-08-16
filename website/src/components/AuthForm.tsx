import { useState } from 'react';
import { t } from '../i18n';

/**
 * Sign-in form (website-plan.md §5). No self-signup: tenant records are
 * created by the Paddle webhook at first purchase, so the only web flow is
 * OTP sign-in for existing tenants:
 *
 *   request-otp → verify-otp → session token
 *
 * The session token stays in sessionStorage (the v1 choice from §11; the
 * hardening follow-up is an httpOnly cookie). Degrades to a "not configured"
 * notice when PUBLIC_LICENSE_API_URL is unset.
 */
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

interface Props {
  locale: string;
}

type Step = 'form' | 'code';

export default function AuthForm({ locale }: Props) {
  const [step, setStep] = useState<Step>('form');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  if (!API) {
    return <p className="rounded-md border border-ink/10 p-4 text-sm text-muted">{t(locale, 'login.notConfigured')}</p>;
  }

  const requestOtp = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await fetch(`${API}/api/v1/web/request-otp`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email }),
      });
      if (!res.ok) throw new Error('request-otp failed');
      setStep('code');
    } catch {
      setError(t(locale, 'login.errorSend'));
    } finally {
      setLoading(false);
    }
  };

  const verifyOtp = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await fetch(`${API}/api/v1/web/verify-otp`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, code }),
      });
      if (!res.ok) throw new Error('verify-otp failed');
      const data = (await res.json()) as { token?: string };
      if (!data.token) throw new Error('no token');
      sessionStorage.setItem('oz_session', data.token);
      window.location.href = `/${locale}/account`;
    } catch {
      setError(t(locale, 'login.errorVerify'));
    } finally {
      setLoading(false);
    }
  };

  const inputClass =
    'w-full rounded-md border border-ink/10 bg-primary px-3 py-2 text-sm text-ink outline-none transition focus:border-accent';

  if (step === 'code') {
    return (
      <div className="mx-auto w-full max-w-sm rounded-xl border border-ink/10 bg-surface/40 p-6">
        <p className="mb-4 text-sm text-muted">{t(locale, 'login.codeSent')}</p>
        <form onSubmit={verifyOtp} className="space-y-4" aria-label={t(locale, 'login.title')}>
          <label className="block">
            <span className="mb-1 block text-sm text-muted">{t(locale, 'login.code')}</span>
            <input
              type="text"
              inputMode="numeric"
              required
              autoComplete="one-time-code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder={t(locale, 'login.codePlaceholder')}
              className={inputClass}
            />
          </label>
          {error && <p className="text-sm text-link" role="alert">{error}</p>}
          <button
            type="submit"
            disabled={loading}
            className="w-full rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
          >
            {loading ? '…' : t(locale, 'login.verify')}
          </button>
          <button
            type="button"
            onClick={() => setStep('form')}
            className="w-full text-center text-xs text-muted transition hover:text-ink"
          >
            {t(locale, 'login.backToEmail')}
          </button>
        </form>
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-sm rounded-xl border border-ink/10 bg-surface/40 p-6">
      <form onSubmit={requestOtp} className="space-y-4" aria-label={t(locale, 'login.title')}>
        <label className="block">
          <span className="mb-1 block text-sm text-muted">{t(locale, 'login.email')}</span>
          <input
            type="email"
            required
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={t(locale, 'login.emailPlaceholder')}
            className={inputClass}
          />
        </label>
        {error && <p className="text-sm text-link" role="alert">{error}</p>}
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
        >
          {loading ? '…' : t(locale, 'login.sendCode')}
        </button>
        <p className="text-xs text-muted">{t(locale, 'login.otpNote')}</p>
      </form>
    </div>
  );
}
