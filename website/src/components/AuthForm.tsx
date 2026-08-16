import { useState } from 'react';
import { t } from '../i18n';

/**
 * Register-or-login form (website-plan.md §5). Payment is register-first:
 * request-otp self-signs a new ACTIVE tenant on first use (the server
 * never reveals whether the account pre-existed), so the OTP flow covers
 * both new accounts and returning tenants:
 *
 *   request-otp → verify-otp → session token (+ cached email)
 *
 * Returning tenants who set a password (account dashboard → set-password)
 * can instead sign in directly:
 *
 *   login (email + password) → session token (+ cached email)
 *
 * The two modes are tabs on the same card; OTP is the default because
 * register-first visitors have no password yet. The session token stays in
 * sessionStorage (the v1 choice from §11; the hardening follow-up is an
 * httpOnly cookie). After auth the user is sent to ?next= (e.g. back to
 * the pricing page to continue checkout) or the account dashboard by
 * default. Degrades to a "not configured" notice when PUBLIC_LICENSE_API_URL
 * is unset.
 */
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

interface Props {
  locale: string;
}

type Mode = 'password' | 'otp';
type Step = 'form' | 'code';

export default function AuthForm({ locale }: Props) {
  const [mode, setMode] = useState<Mode>('otp');
  const [step, setStep] = useState<Step>('form');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  if (!API) {
    return <p className="rounded-md border border-ink/10 p-4 text-sm text-muted">{t(locale, 'login.notConfigured')}</p>;
  }

  const redirectAfterAuth = () => {
    // Honor ?next= (e.g. back to pricing after the sign-in gate) but
    // only for same-site paths — never a protocol-relative or external
    // URL (open-redirect guard).
    const next = new URLSearchParams(window.location.search).get('next');
    const target = next && next.startsWith('/') && !next.startsWith('//') ? next : `/${locale}/account`;
    window.location.href = target;
  };

  const switchMode = (next: Mode) => {
    setMode(next);
    setError('');
  };

  const loginPassword = async (e: { preventDefault(): void }) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await fetch(`${API}/api/v1/web/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password }),
      });
      if (!res.ok) throw new Error('login failed');
      const data = (await res.json()) as { token?: string };
      if (!data.token) throw new Error('no token');
      sessionStorage.setItem('oz_session', data.token);
      // Cache the verified email so checkout can prefill it without a
      // round-trip to /me (see paddle.getSessionEmail).
      sessionStorage.setItem('oz_email', email);
      redirectAfterAuth();
    } catch {
      setError(t(locale, 'login.errorLogin'));
    } finally {
      setLoading(false);
    }
  };

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
      sessionStorage.setItem('oz_email', email);
      redirectAfterAuth();
    } catch {
      setError(t(locale, 'login.errorVerify'));
    } finally {
      setLoading(false);
    }
  };

  const inputClass =
    'w-full rounded-md border border-ink/10 bg-primary px-3 py-2 text-sm text-ink outline-none transition focus:border-accent';

  const tabClass = (active: boolean) =>
    `rounded-md px-3 py-1.5 text-sm font-medium transition ${
      active ? 'bg-primary text-ink shadow-sm' : 'text-muted hover:text-ink'
    }`;

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
        role="tablist"
        aria-label={t(locale, 'login.title')}
        className="mb-5 grid grid-cols-2 gap-1 rounded-lg bg-ink/10 p-1"
      >
        <button type="button" role="tab" aria-selected={mode === 'otp'} onClick={() => switchMode('otp')} className={tabClass(mode === 'otp')}>
          {t(locale, 'login.tabEmailCode')}
        </button>
        <button type="button" role="tab" aria-selected={mode === 'password'} onClick={() => switchMode('password')} className={tabClass(mode === 'password')}>
          {t(locale, 'login.tabPassword')}
        </button>
      </div>

      {mode === 'password' ? (
        <form onSubmit={loginPassword} className="space-y-4" aria-label={t(locale, 'login.tabPassword')}>
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
          <label className="block">
            <span className="mb-1 block text-sm text-muted">{t(locale, 'login.password')}</span>
            <input
              type="password"
              required
              autoComplete="current-password"
              minLength={8}
              maxLength={72}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t(locale, 'login.passwordPlaceholder')}
              className={inputClass}
            />
          </label>
          {error && <p className="text-sm text-link" role="alert">{error}</p>}
          <button
            type="submit"
            disabled={loading}
            className="w-full rounded-md bg-accent px-4 py-2.5 text-sm font-semibold text-black transition hover:opacity-90 disabled:opacity-60"
          >
            {loading ? '…' : t(locale, 'login.signIn')}
          </button>
          <p className="text-xs text-muted">{t(locale, 'login.forgotPassword')}</p>
        </form>
      ) : (
        <form onSubmit={requestOtp} className="space-y-4" aria-label={t(locale, 'login.tabEmailCode')}>
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
          <p className="text-xs text-muted">{t(locale, 'login.newAccount')}</p>
        </form>
      )}
    </div>
  );
}
