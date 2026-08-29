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
    const codeVal = document.getElementById('otp-code') ? document.getElementById('otp-code').value.trim() : '';
    const isCodeActive = otpGroup && !otpGroup.classList.contains('hidden');
    if (isCodeActive) {
      if (loginBtn) loginBtn.textContent = 'Verify Code & Sign In';
    } else {
      if (otpGroup) otpGroup.classList.add('hidden');
      if (loginBtn) loginBtn.textContent = 'Send Verification Code';
    }
  } else {
    if (tabOtp) tabOtp.classList.remove('active');
    if (tabPwd) tabPwd.classList.add('active');
    if (pwdGroup) pwdGroup.classList.remove('hidden');
    if (otpGroup) otpGroup.classList.add('hidden');
    if (cd) cd.classList.add('hidden');
    if (loginBtn) loginBtn.textContent = 'Sign In with Password';
  }
}

// Make setAuthMode available globally
window.setAuthMode = setAuthMode;

function startOtpCooldown(seconds = 60) {
  let sec = seconds;
  const cd = document.getElementById('otp-cooldown');
  if (!cd) return;
  cd.classList.remove('hidden');
  cd.textContent = `Resend code in ${sec}s`;
  if (otpTimer) clearInterval(otpTimer);
  otpTimer = setInterval(() => {
    sec--;
    if (sec <= 0) {
      clearInterval(otpTimer);
      cd.textContent = 'Did not receive code? Click below to resend.';
      const loginBtn = document.getElementById('login-btn');
      if (loginBtn && currentMode === 'otp') {
        // Allow resend
      }
    } else {
      cd.textContent = `Resend code in ${sec}s`;
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

async function handleLogin() {
  hideError();
  hideSuccess();

  const email = document.getElementById('email').value.trim();
  if (!email) {
    showError('Please enter your email address');
    document.getElementById('email').focus();
    return;
  }

  if (currentMode === 'otp') {
    const otpGroup = document.getElementById('otp-group');
    const isCodePromptVisible = otpGroup && !otpGroup.classList.contains('hidden');

    if (!isCodePromptVisible) {
      // Step 1: Request OTP code
      setLoading(true, 'Sending verification code…');
      try {
        const res = await fetch(API + '/api/v1/web/request-otp', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email })
        });
        const body = await res.json();
        if (res.status === 200) {
          otpGroup.classList.remove('hidden');
          document.getElementById('login-btn').textContent = 'Verify Code & Sign In';
          showSuccess('✓ Verification code sent! Please check your email inbox (and spam folder).');
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
          showError(body.error || 'Access denied: origin not allowed');
          setLoading(false);
          return;
        }
        if (res.status === 503) {
          showError('Email delivery is not configured on server');
          setLoading(false);
          return;
        }
        showError(body.error || 'Failed to send verification code');
      } catch (err) {
        showError('Could not connect to authentication server');
      }
      setLoading(false);
      return;
    }

    // Step 2: Verify OTP code
    const code = document.getElementById('otp-code').value.trim();
    if (!code) {
      showError('Please enter the 6-digit code from your email');
      document.getElementById('otp-code').focus();
      return;
    }
    setLoading(true, 'Verifying code…');
    try {
      const res = await fetch(API + '/api/v1/web/verify-otp', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, code })
      });
      const body = await res.json();
      if (res.status === 200 && body.token) {
        showSuccess('✓ Code verified! Signing in…');
        await exchangeForCode(body.token);
        return;
      }
      if (res.status === 429) {
        showLockoutCountdown(body.retry_after || 60);
        setLoading(false);
        return;
      }
      if (res.status === 403) {
        showError(body.error || 'Access denied: origin not allowed');
        setLoading(false);
        return;
      }
      showError(body.error || 'Invalid or expired verification code');
    } catch (err) {
      showError('Could not connect to authentication server');
    }
    setLoading(false);
    return;
  }

  // Password login
  const password = document.getElementById('password').value;
  if (!password) {
    showError('Please enter your password');
    document.getElementById('password').focus();
    return;
  }
  setLoading(true, 'Signing in…');
  try {
    const res = await fetch(API + '/api/v1/web/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password })
    });
    const body = await res.json();
    if (res.status === 200 && body.token) {
      showSuccess('✓ Signing in…');
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
      showError(body.error || 'Access denied: origin not allowed');
      setLoading(false);
      return;
    }
    showError(body.error || 'Invalid email or password');
  } catch (err) {
    showError('Could not connect to authentication server');
  }
  setLoading(false);
}

// ── Lockout countdown (429 with retry_after) ─────────────────
function showLockoutCountdown(seconds) {
  const btn = document.getElementById('login-btn');
  if (!btn) return;
  btn.disabled = true;
  let remaining = seconds;
  btn.textContent = `Try again in ${remaining}s`;
  const timer = setInterval(() => {
    remaining--;
    if (remaining <= 0) {
      clearInterval(timer);
      btn.disabled = false;
      btn.textContent = currentMode === 'otp' ? 'Send Verification Code' : 'Sign In';
      return;
    }
    btn.textContent = `Try again in ${remaining}s`;
  }, 1000);
}

function handleFormSubmit(e) {
  if (e && e.preventDefault) e.preventDefault();
  handleLogin();
}

window.handleFormSubmit = handleFormSubmit;
window.handleLogin = handleLogin;

// Initialize default mode
setAuthMode('otp');
