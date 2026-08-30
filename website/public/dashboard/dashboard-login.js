// Dashboard login flow — mirrors admin login.js but for the user dashboard.
// API base: relative path when hosted on ozpos.my.id subdomains (Worker API
// proxy) to eliminate CORS; fall back to direct backend URL locally.
const isSubdomain = window.location.hostname.endsWith('ozpos.my.id');
const API = isSubdomain
  ? ''
  : ((window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id');

let currentMode = 'otp';
let otpTimer = null;
let isSubmitting = false;

function showError(msg) {
  hideSuccess();
  const e = document.getElementById('error-msg');
  if (e) { e.textContent = msg; e.classList.remove('hidden'); }
}
function hideError() {
  const e = document.getElementById('error-msg');
  if (e) e.classList.add('hidden');
}
function showSuccess(msg) {
  hideError();
  const e = document.getElementById('success-msg');
  if (e) { e.textContent = msg; e.classList.remove('hidden'); }
}
function hideSuccess() {
  const e = document.getElementById('success-msg');
  if (e) e.classList.add('hidden');
}
function setLoading(on, label) {
  const form = document.getElementById('login-form');
  const loading = document.getElementById('loading-state');
  const btn = document.getElementById('login-btn');
  if (form) form.style.display = on ? 'none' : 'block';
  if (loading) loading.style.display = on ? 'block' : 'none';
  if (btn) btn.disabled = on;
  if (btn && label) btn.textContent = label;
}

// ── Mode tabs (Email Code / Password) ─────────────────────────
function setAuthMode(mode) {
  currentMode = mode;
  hideError(); hideSuccess();
  const tabOtp = document.getElementById('tab-otp');
  const tabPwd = document.getElementById('tab-password');
  const pwdGroup = document.getElementById('password-group');
  const otpGroup = document.getElementById('otp-group');
  const loginBtn = document.getElementById('login-btn');

  if (mode === 'otp') {
    if (tabOtp) tabOtp.classList.add('active');
    if (tabPwd) tabPwd.classList.remove('active');
    if (pwdGroup) pwdGroup.classList.add('hidden');
    const codeVal = document.getElementById('otp-code') ? document.getElementById('otp-code').value.trim() : '';
    const isCodeActive = otpGroup && !otpGroup.classList.contains('hidden');
    if (isCodeActive && codeVal) {
      if (loginBtn) loginBtn.textContent = t('login.verifyCode');
    } else {
      if (otpGroup) otpGroup.classList.add('hidden');
      if (loginBtn) loginBtn.textContent = t('login.sendCode');
    }
  } else {
    if (tabOtp) tabOtp.classList.remove('active');
    if (tabPwd) tabPwd.classList.add('active');
    if (pwdGroup) pwdGroup.classList.remove('hidden');
    if (otpGroup) otpGroup.classList.add('hidden');
    if (loginBtn) loginBtn.textContent = t('login.signIn');
  }
}

function startOtpCooldown(seconds) {
  const cd = document.getElementById('otp-cooldown');
  const btn = document.getElementById('login-btn');
  if (!cd) return;
  cd.classList.remove('hidden');
  let remaining = seconds;
  cd.textContent = t('login.resendIn') + remaining + t('login.seconds');
  if (otpTimer) clearInterval(otpTimer);
  otpTimer = setInterval(() => {
    remaining--;
    if (remaining <= 0) {
      clearInterval(otpTimer);
      cd.classList.add('hidden');
      if (btn) btn.disabled = false;
    } else {
      cd.textContent = t('login.resendIn') + remaining + t('login.seconds');
    }
  }, 1000);
}

// ── Exchange token for a one-time code (hardening F1) ─────────
async function exchangeForCode(token) {
  const res = await fetch(API + '/api/v1/web/exchange-issue', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token }
  });
  if (!res.ok) throw new Error('exchange failed');
  const body = await res.json();
  window.location.href = '/?code=' + encodeURIComponent(body.code);
}

async function handleLogin() {
  if (isSubmitting) return;
  isSubmitting = true;
  hideError(); hideSuccess();

  try {
    const email = document.getElementById('email').value.trim();
    if (!email) {
      showError(t('login.enterEmail'));
      document.getElementById('email').focus();
      return;
    }

    if (currentMode === 'otp') {
      const otpGroup = document.getElementById('otp-group');
      const isCodePromptVisible = otpGroup && !otpGroup.classList.contains('hidden');

      if (!isCodePromptVisible) {
        // Step 1: Request OTP code
        setLoading(true, t('login.sendingCode'));
        try {
          const res = await fetch(API + '/api/v1/web/request-otp', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email })
          });
          const body = await res.json();
          if (res.status === 200) {
            otpGroup.classList.remove('hidden');
            document.getElementById('login-btn').textContent = t('login.verifyCode');
            showSuccess(t('login.codeSent'));
            startOtpCooldown(60);
            setLoading(false);
            return;
          }
          if (res.status === 429) {
            showLockoutCountdown(body.retry_after || 60);
            setLoading(false);
            return;
          }
          if (res.status === 403) {
            showError(body.error || t('login.accessDeniedOrigin'));
            setLoading(false);
            return;
          }
          if (res.status === 503) {
            showError(t('login.emailDeliveryNotConfigured'));
            setLoading(false);
            return;
          }
          showError(body.error || t('login.failedToSendCode'));
        } catch (err) {
          showError(t('login.couldNotConnect'));
        }
        setLoading(false);
        return;
      }

      // Step 2: Verify OTP code
      const code = document.getElementById('otp-code').value.trim();
      if (!code) {
        showError(t('login.enterCode'));
        document.getElementById('otp-code').focus();
        return;
      }
      setLoading(true, t('login.verifyingCode'));
      try {
        const res = await fetch(API + '/api/v1/web/verify-otp', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, code })
        });
        const body = await res.json();
        if (res.status === 200 && body.token) {
          showSuccess(t('login.codeVerified'));
          await exchangeForCode(body.token);
          return;
        }
        if (res.status === 429) {
          showLockoutCountdown(body.retry_after || 60);
          setLoading(false);
          return;
        }
        if (res.status === 403) {
          showError(body.error || t('login.accessDeniedOrigin'));
          setLoading(false);
          return;
        }
        showError(body.error || t('login.invalidOrExpiredCodeShort'));
      } catch (err) {
        showError(t('login.couldNotConnect'));
      }
      setLoading(false);
      return;
    }

    // Password mode
    const password = document.getElementById('password').value;
    if (!password) {
      showError(t('login.enterPassword'));
      document.getElementById('password').focus();
      return;
    }
    setLoading(true, t('login.signingIn'));
    try {
      const res = await fetch(API + '/api/v1/web/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password })
      });
      const body = await res.json();
      if (res.status === 200 && body.token) {
        showSuccess(t('login.signingInNow'));
        await exchangeForCode(body.token);
        return;
      }
      if (res.status === 429) {
        showLockoutCountdown(body.retry_after || 60);
        setLoading(false);
        return;
      }
      if (res.status === 403) {
        showError(body.error || t('login.accessDeniedOrigin'));
        setLoading(false);
        return;
      }
      showError(body.error || t('login.invalidEmailOrPassword'));
    } catch (err) {
      showError(t('login.couldNotConnect'));
    }
    setLoading(false);
  } finally {
    isSubmitting = false;
  }
}

// ── Lockout countdown (429 with retry_after) ─────────────────
function showLockoutCountdown(seconds) {
  const btn = document.getElementById('login-btn');
  if (!btn) return;
  btn.disabled = true;
  let remaining = seconds;
  btn.textContent = t('login.tryAgainIn') + remaining + t('login.seconds');
  const timer = setInterval(() => {
    remaining--;
    if (remaining <= 0) {
      clearInterval(timer);
      btn.disabled = false;
      btn.textContent = currentMode === 'otp' ? t('login.sendCode') : t('login.signIn');
      return;
    }
    btn.textContent = t('login.tryAgainIn') + remaining + t('login.seconds');
  }, 1000);
}

// ── Wire events (no inline handlers — strict CSP) ─────────────
const form = document.getElementById('auth-form');
if (form) {
  form.addEventListener('submit', (e) => { e.preventDefault(); handleLogin(); });
}
const tabOtp = document.getElementById('tab-otp');
const tabPwd = document.getElementById('tab-password');
if (tabOtp) tabOtp.addEventListener('click', () => setAuthMode('otp'));
if (tabPwd) tabPwd.addEventListener('click', () => setAuthMode('password'));

// Theme toggle (theme.js is loaded in <head>)
const themeToggle = document.getElementById('theme-toggle');
if (themeToggle) {
  themeToggle.addEventListener('click', () => {
    if (window.__ozDashboardTheme) { window.__ozDashboardTheme.toggle(); }
  });
}

// Initialize default mode
setAuthMode('otp');