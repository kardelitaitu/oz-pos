import { useState } from 'react';
import { t } from '../i18n';

/**
 * Sign in / Create account (website-plan.md §5). Two tabs share one OTP flow:
 *
 *  - Sign in:      request-otp  → verify-otp  → session token
 *  - Create acct:  register     → verify-otp  → session token
 *
 * The session token stays in sessionStorage (the v1 choice from §11; the
 * hardening follow-up is an httpOnly cookie). Degrades to a "not configured"
 * notice when PUBLIC_LICENSE_API_URL is unset.
 */
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

interface Props {
  locale: string;
}

type Mode = 'signin' | 'signup';
type Step = 'form' | 'code';

export default function AuthForm({ locale }: Props) {
  const [mode, setMode] = useState<Mode>('signin');
  const [step, setStep] = useState<Step>('form');
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  if (!API) {
    return <p className="rounded-md border border-ink/10 p-4 text-sm text-muted">{t(locale, 'login.notConfigured')}</p>;
  }

  const submitForm = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const path = mode === 'signup' ? '/api/v1/web/register' : '/api/v1/web/request-otp';
      const res = await fetch(`${API}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(mode === 'signup' ? { name, email } : { email }),
      });
      if (!res.ok) throw new Error(`${path} failed`);
      setStep('code');
    } catch {
      setError(t(locale, mode === 'signup' ? 'login.errorSignup' : 'login.errorSend'));
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

  const switchMode = (next: Mode) => {
    setMode(next);
    setStep('form');
    setError('');
    setCode('');
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
      <div
        role="group"
        aria-label={t(locale, 'login.title')}
        className="mb-6 grid grid-cols-2 rounded-lg border border-ink/10 bg-primary p-1"
      >
        <button
          type="button"
          onClick={() => switchMode('signin')}
          aria-pressed={mode === 'signin'}
          className={`rounded-md px-3 py-1.5 text-sm font-medium transition ${
            mode === 'signin' ? 'bg-accent text-black' : 'text-muted hover:text-ink'
          }`}
        >
          {t(locale, 'login.signInTab')}
        </button>
        <button
          type="button"
          onClick={() => switchMode('signup')}
          aria-pressed={mode === 'signup'}
          className={`rounded-md px-3 py-1.5 text-sm font-medium transition ${
            mode === 'signup' ? 'bg-accent text-black' : 'text-muted hover:text-ink'
          }`}
        >
          {t(locale, 'login.signUpTab')}
        </button>
      </div>

      <form onSubmit={submitForm} className="space-y-4" aria-label={mode === 'signup' ? t(locale, 'login.signUpTab') : t(locale, 'login.signInTab')}>
        {mode === 'signup' && (
          <label className="block">
            <span className="mb-1 block text-sm text-muted">{t(locale, 'login.name')}</span>
            <input
              type="text"
              required
              maxLength={100}
              autoComplete="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t(locale, 'login.namePlaceholder')}
              className={inputClass}
            />
          </label>
        )}
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
          {loading ? '…' : t(locale, mode === 'signup' ? 'login.createAccount' : 'login.sendCode')}
        </button>
        <p className="text-xs text-muted">{t(locale, 'login.otpNote')}</p>
      </form>
    </div>
  );
}
