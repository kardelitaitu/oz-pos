import { useState, useEffect, useCallback } from 'react';
import { t } from '../i18n';
import { licenseApiUrl } from './runtime-config';

interface UseAuthOptions {
  locale: string;
  onAuthSuccess?: (token: string, email: string) => void;
}

export function useAuth({ locale, onAuthSuccess }: UseAuthOptions) {
  const API = licenseApiUrl();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [otpSentAt, setOtpSentAt] = useState<number | null>(null);
  const [resendCooldown, setResendCooldown] = useState(0);
  const [resendSuccess, setResendSuccess] = useState(false);

  // Countdown timer for OTP resend (120 seconds)
  useEffect(() => {
    if (!otpSentAt) return;
    const tick = () => {
      const elapsed = Math.floor((Date.now() - otpSentAt) / 1000);
      const remaining = Math.max(0, 120 - elapsed);
      setResendCooldown(remaining);
      if (remaining <= 0) clearInterval(id);
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [otpSentAt]);

  const triggerResendSuccess = useCallback(() => {
    setResendSuccess(true);
    setTimeout(() => setResendSuccess(false), 4000);
  }, []);

  const handleApiError = useCallback(
    (res: Response, body: { error?: string }, fallbackKey: string) => {
      if (res.status === 429) {
        setError(t(locale, 'login.errorRateLimit'));
      } else if (res.status === 403) {
        setError(t(locale, 'login.errorCors'));
      } else if (res.status === 503) {
        setError(t(locale, 'login.errorSmtp'));
      } else {
        const msg = body.error;
        setError(msg ? `${t(locale, fallbackKey)} (${msg})` : t(locale, fallbackKey));
      }
    },
    [locale]
  );

  const requestOtp = useCallback(
    async (email: string, isResend = false): Promise<boolean> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/request-otp`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: sanitizedEmail }),
        });
        if (!res.ok) {
          const body = (await res.json().catch(() => ({}))) as { error?: string };
          handleApiError(res, body, 'login.errorSend');
          return false;
        }
        setOtpSentAt(Date.now());
        if (isResend) triggerResendSuccess();
        return true;
      } catch {
        setError(t(locale, 'login.errorSend'));
        return false;
      } finally {
        setLoading(false);
      }
    },
    [API, handleApiError, locale, triggerResendSuccess]
  );

  const verifyOtp = useCallback(
    async (email: string, code: string, region?: string): Promise<{ success: boolean; token?: string }> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/verify-otp`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: sanitizedEmail, code: code.trim() }),
        });
        if (!res.ok) throw new Error('verify-otp failed');
        const data = (await res.json()) as { token?: string };
        if (!data.token) throw new Error('no token');

        sessionStorage.setItem('oz_session', data.token);
        sessionStorage.setItem('oz_email', sanitizedEmail);
        if (region) localStorage.setItem('oz_region', region);

        onAuthSuccess?.(data.token, sanitizedEmail);
        return { success: true, token: data.token };
      } catch {
        setError(t(locale, 'login.errorVerify'));
        return { success: false };
      } finally {
        setLoading(false);
      }
    },
    [API, locale, onAuthSuccess]
  );

  const loginPassword = useCallback(
    async (email: string, password: string): Promise<{ success: boolean; token?: string }> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/login`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: sanitizedEmail, password }),
        });
        if (!res.ok) throw new Error('login failed');
        const data = (await res.json()) as { token?: string };
        if (!data.token) throw new Error('no token');

        sessionStorage.setItem('oz_session', data.token);
        sessionStorage.setItem('oz_email', sanitizedEmail);

        onAuthSuccess?.(data.token, sanitizedEmail);
        return { success: true, token: data.token };
      } catch {
        setError(t(locale, 'login.errorLogin'));
        return { success: false };
      } finally {
        setLoading(false);
      }
    },
    [API, locale, onAuthSuccess]
  );

  const register = useCallback(
    async (email: string, password: string, passwordConfirm: string): Promise<boolean> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/register`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: sanitizedEmail, password, password_confirm: passwordConfirm }),
        });
        if (res.status === 409) {
          setError(t(locale, 'signup.errorExists'));
          return false;
        }
        if (!res.ok) {
          const body = (await res.json().catch(() => ({}))) as { error?: string };
          handleApiError(res, body, 'signup.errorRegister');
          return false;
        }
        setOtpSentAt(Date.now());
        return true;
      } catch {
        setError(t(locale, 'signup.errorRegister'));
        return false;
      } finally {
        setLoading(false);
      }
    },
    [API, handleApiError, locale]
  );

  const requestResetCode = useCallback(
    async (email: string): Promise<{ success: boolean; cooldownUntil?: string }> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/request-password-reset`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email: sanitizedEmail }),
        });
        const data = (await res.json().catch(() => ({}))) as { cooldown_until?: string; error?: string };
        if (!res.ok) {
          handleApiError(res, data, 'login.errorResetRequest');
          return { success: false };
        }
        return { success: true, cooldownUntil: data.cooldown_until };
      } catch {
        setError(t(locale, 'login.errorResetRequest'));
        return { success: false };
      } finally {
        setLoading(false);
      }
    },
    [API, handleApiError, locale]
  );

  const resetPassword = useCallback(
    async (
      email: string,
      code: string,
      password: string,
      passwordConfirm: string
    ): Promise<{ success: boolean; token?: string }> => {
      const sanitizedEmail = email.trim().toLowerCase();
      setError('');
      setLoading(true);
      try {
        const res = await fetch(`${API}/api/v1/web/reset-password`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            email: sanitizedEmail,
            code: code.trim(),
            password,
            password_confirm: passwordConfirm,
          }),
        });
        if (!res.ok) throw new Error('reset-password failed');
        const data = (await res.json()) as { token?: string };
        if (!data.token) throw new Error('no token');

        sessionStorage.setItem('oz_session', data.token);
        sessionStorage.setItem('oz_email', sanitizedEmail);

        onAuthSuccess?.(data.token, sanitizedEmail);
        return { success: true, token: data.token };
      } catch {
        setError(t(locale, 'login.errorReset'));
        return { success: false };
      } finally {
        setLoading(false);
      }
    },
    [API, locale, onAuthSuccess]
  );

  return {
    loading,
    error,
    setError,
    otpSentAt,
    resendCooldown,
    resendSuccess,
    requestOtp,
    verifyOtp,
    loginPassword,
    register,
    requestResetCode,
    resetPassword,
  };
}
