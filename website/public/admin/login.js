// API base: use relative path when hosted on ozpos.my.id subdomains (Worker API proxy)
// to eliminate CORS and in-handler origin restrictions. Fall back to direct backend URL locally.
const isSubdomain = window.location.hostname.endsWith('ozpos.my.id');
const API = isSubdomain
  ? ''
  : ((window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id');

let currentMode = 'otp'; // default to 'otp' since admin accounts start without a password
let otpTimer = null;

function showError(msg) {
  hideSuccess();
  const e = document.getElementById('error-msg');
  if (e) {
    e.textContent = msg;
    e.classList.remove('hidden');
  }
}

function hideError() {
  const e = document.getElementById('error-msg');
  if (e) e.classList.add('hidden');
}

function showSuccess(msg) {
  hideError();
  const e = document.getElementById('success-msg');
  if (e) {
    e.textContent = msg;
    e.classList.remove('hidden');
  }
}

function hideSuccess() {
  const e = document.getElementById('success-msg');
  if (e) e.classList.add('hidden');
}

function setLoading(l, msg) {
  const formEl = document.getElementById('login-form');
  const loadingEl = document.getElementById('loading-state');
  if (formEl) formEl.style.display = l ? 'none' : 'block';
  if (loadingEl) {
    loadingEl.style.display = l ? 'block' : 'none';
    if (msg) {
      const textEl = loadingEl.querySelector('.loading-text');
      if (textEl) textEl.textContent = msg;
    }
  }
}

function setAuthMode(mode) {
  currentMode = mode;
  hideError();
  hideSuccess();

  const tabOtp = document.getElementById('tab-otp');
  const tabPwd = document.getElementById('tab-password');
  const pwdGroup = document.getElementById('password-group');
  const otpGroup = document.getElementById('otp-group');
  const loginBtn = document.getElementById('login-btn');
  const cd = document.getElementById('otp-cooldown');

  if (mode === 'otp') {
    if (tabOtp) tabOtp.classList.add('active');
    if (tabPwd) tabPwd.classList.remove('active');
    if (pwdGroup) pwdGroup.classList.add('hidden');
    
    // Check if code was already sent
    const isCodeActive = otpGroup && !otpGroup.classList.contains('hidden');
    if (isCodeActive) {
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
    if (cd) cd.classList.add('hidden');
    if (loginBtn) loginBtn.textContent = t('login.signInPassword');
  }
}

function startOtpCooldown(seconds = 60) {
  let sec = seconds;
  const cd = document.getElementById('otp-cooldown');
  if (!cd) return;
  cd.classList.remove('hidden');
  cd.textContent = t('login.resendIn') + sec + t('login.seconds');
  if (otpTimer) clearInterval(otpTimer);
  otpTimer = setInterval(() => {
    sec--;
    if (sec <= 0) {
      clearInterval(otpTimer);
      cd.textContent = t('login.resendPrompt');
      const loginBtn = document.getElementById('login-btn');
      if (loginBtn && currentMode === 'otp') {
        // Allow resend
      }
    } else {
      cd.textContent = t('login.resendIn') + sec + t('login.seconds');
    }
  }, 1000);
}

// ── Exchange token for a one-time code (hardening F1) ──────────
async function exchangeForCode(token) {
  const res = await fetch(API + '/api/v1/web/exchange-issue', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token }
  });
  if (!res.ok) throw new Error('exchange failed');
  const body = await res.json();
  window.location.href = '/?code=' + encodeURIComponent(body.code);
}

let isSubmitting = false;

async function handleLogin() {
  if (isSubmitting) return;
  isSubmitting = true;
  hideError();
  hideSuccess();

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
            const otpInput = document.getElementById('otp-code');
            if (otpInput) otpInput.focus();
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
        showError(body.error || t('login.invalidOrExpiredCode'));
      } catch (err) {
        showError(t('login.couldNotConnect'));
      }
      setLoading(false);
      return;
    }

    // Password login
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
        const retryAfter = body.retry_after || 60;
        showLockoutCountdown(retryAfter);
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
  // B7 fix: delegates to admin-utils.startLockoutCountdown, which keeps
  // one tracked timer per button. The old inline version created a NEW
  // setInterval per 429 without clearing the previous one, so a second
  // rate-limited response left two timers racing: the shorter one
  // re-enabled the button early and the other zombie-rewrote the label.
  startLockoutCountdown(
    btn, seconds,
    function (s) { return t('login.tryAgainIn') + s + t('login.seconds'); },
    function () { return currentMode === 'otp' ? t('login.sendCode') : t('login.signIn'); }
  );
}

// Wire single submit event on form
const form = document.getElementById('auth-form');
if (form) {
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    handleLogin();
  });
}

// Wire mode tabs (no inline handlers — strict CSP)
const tabOtp = document.getElementById('tab-otp');
const tabPwd = document.getElementById('tab-password');
if (tabOtp) tabOtp.addEventListener('click', () => setAuthMode('otp'));
if (tabPwd) tabPwd.addEventListener('click', () => setAuthMode('password'));

// Theme toggle (theme.js is loaded in <head>)
const themeToggle = document.getElementById('theme-toggle');
if (themeToggle) {
  themeToggle.addEventListener('click', () => {
    if (window.__ozAdminTheme) { window.__ozAdminTheme.toggle(); }
  });
}

// Initialize default mode
setAuthMode('otp');
